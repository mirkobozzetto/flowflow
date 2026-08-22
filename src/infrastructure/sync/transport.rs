use super::SyncError;
use snow::params::NoiseParams;
use snow::{Builder, HandshakeState, Keypair, TransportState};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub const NOISE_PARAMS: &str = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s";
pub const PSK_LEN: usize = 32;
pub const MAX_NOISE_MESSAGE: usize = 65535;
pub const NOISE_TAG_LEN: usize = 16;
pub const MAX_PLAINTEXT: usize = MAX_NOISE_MESSAGE - NOISE_TAG_LEN;
pub const MAX_LOGICAL_MESSAGE: usize = 64 * 1024 * 1024;
pub const IO_TIMEOUT: Duration = Duration::from_secs(20);

fn params() -> Result<NoiseParams, SyncError> {
    NOISE_PARAMS
        .parse()
        .map_err(|e| SyncError::Handshake(format!("noise params: {e:?}")))
}

pub fn generate_static_keypair() -> Result<Keypair, SyncError> {
    Builder::new(params()?)
        .generate_keypair()
        .map_err(|e| SyncError::Handshake(format!("keypair: {e}")))
}

pub fn build_initiator(
    local_private: &[u8],
    psk: &[u8; PSK_LEN],
) -> Result<HandshakeState, SyncError> {
    Builder::new(params()?)
        .local_private_key(local_private)
        .map_err(|e| SyncError::Handshake(format!("private key: {e}")))?
        .psk(3, psk)
        .map_err(|e| SyncError::Handshake(format!("psk: {e}")))?
        .build_initiator()
        .map_err(|e| SyncError::Handshake(format!("initiator: {e}")))
}

pub fn build_responder(
    local_private: &[u8],
    psk: &[u8; PSK_LEN],
) -> Result<HandshakeState, SyncError> {
    Builder::new(params()?)
        .local_private_key(local_private)
        .map_err(|e| SyncError::Handshake(format!("private key: {e}")))?
        .psk(3, psk)
        .map_err(|e| SyncError::Handshake(format!("psk: {e}")))?
        .build_responder()
        .map_err(|e| SyncError::Handshake(format!("responder: {e}")))
}

pub(crate) fn write_frame<W: Write>(
    w: &mut W,
    data: &[u8],
) -> Result<(), SyncError> {
    if data.len() > MAX_NOISE_MESSAGE {
        return Err(SyncError::Transport(format!(
            "frame too large: {}",
            data.len()
        )));
    }
    let len = (data.len() as u16).to_be_bytes();
    w.write_all(&len)
        .map_err(|e| SyncError::Transport(format!("frame write len: {e}")))?;
    w.write_all(data)
        .map_err(|e| SyncError::Transport(format!("frame write: {e}")))?;
    w.flush()
        .map_err(|e| SyncError::Transport(format!("frame flush: {e}")))
}

pub(crate) fn read_frame<R: Read>(r: &mut R) -> Result<Vec<u8>, SyncError> {
    let mut len_buf = [0u8; 2];
    r.read_exact(&mut len_buf)
        .map_err(|e| SyncError::Transport(format!("frame read len: {e}")))?;
    let len = u16::from_be_bytes(len_buf) as usize;
    let mut data = vec![0u8; len];
    r.read_exact(&mut data)
        .map_err(|e| SyncError::Transport(format!("frame read: {e}")))?;
    Ok(data)
}

fn verify_remote_static(
    remote: Option<&[u8]>,
    expected: Option<&[u8]>,
) -> Result<(), SyncError> {
    let Some(expected) = expected else {
        return Ok(());
    };
    match remote {
        Some(remote) if remote == expected => Ok(()),
        Some(_) => Err(SyncError::Handshake(
            "remote static key does not match expected fingerprint".into(),
        )),
        None => Err(SyncError::Handshake(
            "remote static key missing after handshake".into(),
        )),
    }
}

pub struct SecureChannel<S> {
    stream: S,
    noise: TransportState,
    // Noise handshake hash, kept for channel binding: a signature over it
    // proves the signer sits on THIS session, not a replayed transcript.
    handshake_hash: Vec<u8>,
}

impl<S: Read + Write> SecureChannel<S> {
    pub fn remote_static(&self) -> Option<Vec<u8>> {
        self.noise.get_remote_static().map(|k| k.to_vec())
    }

    pub fn handshake_hash(&self) -> &[u8] {
        &self.handshake_hash
    }

    pub fn send(&mut self, msg: &[u8]) -> Result<(), SyncError> {
        if msg.len() > MAX_LOGICAL_MESSAGE {
            return Err(SyncError::Transport(format!(
                "message too large: {}",
                msg.len()
            )));
        }
        let mut buf = vec![0u8; MAX_NOISE_MESSAGE];
        let header = (msg.len() as u32).to_be_bytes();
        let n = self.noise.write_message(&header, &mut buf).map_err(|e| {
            SyncError::Transport(format!("encrypt header: {e}"))
        })?;
        write_frame(&mut self.stream, &buf[..n])?;
        for chunk in msg.chunks(MAX_PLAINTEXT) {
            let n = self
                .noise
                .write_message(chunk, &mut buf)
                .map_err(|e| SyncError::Transport(format!("encrypt: {e}")))?;
            write_frame(&mut self.stream, &buf[..n])?;
        }
        Ok(())
    }

    pub fn recv(&mut self) -> Result<Vec<u8>, SyncError> {
        let mut buf = vec![0u8; MAX_NOISE_MESSAGE];
        let frame = read_frame(&mut self.stream)?;
        let n = self.noise.read_message(&frame, &mut buf).map_err(|e| {
            SyncError::Transport(format!("decrypt header: {e}"))
        })?;
        if n != 4 {
            return Err(SyncError::Transport(format!(
                "bad message header length: {n}"
            )));
        }
        let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        if len > MAX_LOGICAL_MESSAGE {
            return Err(SyncError::Transport(format!(
                "incoming message too large: {len}"
            )));
        }
        let mut msg = Vec::with_capacity(len);
        while msg.len() < len {
            let frame = read_frame(&mut self.stream)?;
            let n = self
                .noise
                .read_message(&frame, &mut buf)
                .map_err(|e| SyncError::Transport(format!("decrypt: {e}")))?;
            if n == 0 || msg.len() + n > len {
                return Err(SyncError::Transport(
                    "message frame makes no progress or overflows".into(),
                ));
            }
            msg.extend_from_slice(&buf[..n]);
        }
        Ok(msg)
    }
}

pub fn initiator_handshake<S: Read + Write>(
    mut stream: S,
    local_private: &[u8],
    psk: &[u8; PSK_LEN],
    expected_remote_static: Option<&[u8]>,
) -> Result<SecureChannel<S>, SyncError> {
    let mut hs = build_initiator(local_private, psk)?;
    let mut buf = vec![0u8; MAX_NOISE_MESSAGE];
    let mut read = vec![0u8; MAX_NOISE_MESSAGE];

    let n = hs
        .write_message(&[], &mut buf)
        .map_err(|e| SyncError::Handshake(format!("msg1 write: {e}")))?;
    write_frame(&mut stream, &buf[..n])?;

    let frame = read_frame(&mut stream)?;
    hs.read_message(&frame, &mut read)
        .map_err(|e| SyncError::Handshake(format!("msg2 read: {e}")))?;

    // Verify the responder fingerprint BEFORE sending msg3, which carries our
    // own static key: abort against a wrong peer without disclosing our identity.
    verify_remote_static(hs.get_remote_static(), expected_remote_static)?;

    let n = hs
        .write_message(&[], &mut buf)
        .map_err(|e| SyncError::Handshake(format!("msg3 write: {e}")))?;
    write_frame(&mut stream, &buf[..n])?;

    let handshake_hash = hs.get_handshake_hash().to_vec();
    let noise = hs
        .into_transport_mode()
        .map_err(|e| SyncError::Transport(format!("init transport: {e}")))?;
    Ok(SecureChannel {
        stream,
        noise,
        handshake_hash,
    })
}

pub fn responder_handshake<S: Read + Write>(
    mut stream: S,
    local_private: &[u8],
    psk: &[u8; PSK_LEN],
    expected_remote_static: Option<&[u8]>,
) -> Result<SecureChannel<S>, SyncError> {
    let mut hs = build_responder(local_private, psk)?;
    let mut buf = vec![0u8; MAX_NOISE_MESSAGE];
    let mut read = vec![0u8; MAX_NOISE_MESSAGE];

    let frame = read_frame(&mut stream)?;
    hs.read_message(&frame, &mut read)
        .map_err(|e| SyncError::Handshake(format!("msg1 read: {e}")))?;

    let n = hs
        .write_message(&[], &mut buf)
        .map_err(|e| SyncError::Handshake(format!("msg2 write: {e}")))?;
    write_frame(&mut stream, &buf[..n])?;

    let frame = read_frame(&mut stream)?;
    hs.read_message(&frame, &mut read)
        .map_err(|e| SyncError::Handshake(format!("msg3 read: {e}")))?;

    verify_remote_static(hs.get_remote_static(), expected_remote_static)?;
    let handshake_hash = hs.get_handshake_hash().to_vec();
    let noise = hs
        .into_transport_mode()
        .map_err(|e| SyncError::Transport(format!("resp transport: {e}")))?;
    Ok(SecureChannel {
        stream,
        noise,
        handshake_hash,
    })
}

pub fn connect_tcp(host: &str, port: u16) -> Result<TcpStream, SyncError> {
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| {
            SyncError::Transport(format!("resolve {host}:{port}: {e}"))
        })?
        .next()
        .ok_or_else(|| {
            SyncError::Transport(format!("no address for {host}:{port}"))
        })?;
    let stream = TcpStream::connect_timeout(&addr, IO_TIMEOUT)
        .map_err(|e| SyncError::Transport(format!("connect {addr}: {e}")))?;
    configure_stream(&stream)?;
    Ok(stream)
}

pub fn connect_secure(
    host: &str,
    port: u16,
    local_private: &[u8],
    psk: &[u8; PSK_LEN],
    expected_remote_static: Option<&[u8]>,
) -> Result<SecureChannel<TcpStream>, SyncError> {
    let stream = connect_tcp(host, port)?;
    initiator_handshake(stream, local_private, psk, expected_remote_static)
}

pub fn accept_secure(
    stream: TcpStream,
    local_private: &[u8],
    psk: &[u8; PSK_LEN],
    expected_remote_static: Option<&[u8]>,
) -> Result<SecureChannel<TcpStream>, SyncError> {
    configure_stream(&stream)?;
    responder_handshake(stream, local_private, psk, expected_remote_static)
}

pub(crate) fn configure_stream(stream: &TcpStream) -> Result<(), SyncError> {
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|e| SyncError::Transport(format!("read timeout: {e}")))?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|e| SyncError::Transport(format!("write timeout: {e}")))?;
    stream
        .set_nodelay(true)
        .map_err(|e| SyncError::Transport(format!("nodelay: {e}")))
}

pub fn inmemory_handshake_with_psks(
    initiator_psk: &[u8; PSK_LEN],
    responder_psk: &[u8; PSK_LEN],
    plaintext: &[u8],
) -> Result<Vec<u8>, SyncError> {
    let init_kp = generate_static_keypair()?;
    let resp_kp = generate_static_keypair()?;

    let mut initiator = build_initiator(&init_kp.private, initiator_psk)?;
    let mut responder = build_responder(&resp_kp.private, responder_psk)?;

    let mut buf = vec![0u8; MAX_NOISE_MESSAGE];
    let mut read = vec![0u8; MAX_NOISE_MESSAGE];

    let n = initiator
        .write_message(&[], &mut buf)
        .map_err(|e| SyncError::Handshake(format!("msg1 write: {e}")))?;
    responder
        .read_message(&buf[..n], &mut read)
        .map_err(|e| SyncError::Handshake(format!("msg1 read: {e}")))?;

    let n = responder
        .write_message(&[], &mut buf)
        .map_err(|e| SyncError::Handshake(format!("msg2 write: {e}")))?;
    initiator
        .read_message(&buf[..n], &mut read)
        .map_err(|e| SyncError::Handshake(format!("msg2 read: {e}")))?;

    let n = initiator
        .write_message(&[], &mut buf)
        .map_err(|e| SyncError::Handshake(format!("msg3 write: {e}")))?;
    responder
        .read_message(&buf[..n], &mut read)
        .map_err(|e| SyncError::Handshake(format!("msg3 read: {e}")))?;

    let mut initiator = initiator
        .into_transport_mode()
        .map_err(|e| SyncError::Transport(format!("init transport: {e}")))?;
    let mut responder = responder
        .into_transport_mode()
        .map_err(|e| SyncError::Transport(format!("resp transport: {e}")))?;

    let n = initiator
        .write_message(plaintext, &mut buf)
        .map_err(|e| SyncError::Transport(format!("encrypt: {e}")))?;
    let m = responder
        .read_message(&buf[..n], &mut read)
        .map_err(|e| SyncError::Transport(format!("decrypt: {e}")))?;

    Ok(read[..m].to_vec())
}

pub fn inmemory_handshake_roundtrip(
    plaintext: &[u8],
) -> Result<Vec<u8>, SyncError> {
    let psk = super::peers::generate_psk()?;
    inmemory_handshake_with_psks(&psk, &psk, plaintext)
}

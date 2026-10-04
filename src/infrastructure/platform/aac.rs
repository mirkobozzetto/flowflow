//! WAV to AAC (.m4a) with AVFoundation: a recording sent to the cloud shrinks
//! about tenfold without bundling a codec.

use objc2::runtime::AnyObject;
use objc2::AnyThread;
use objc2_avf_audio::{
    AVAudioCommonFormat, AVAudioFile, AVAudioPCMBuffer, AVEncoderBitRateKey,
    AVFormatIDKey, AVNumberOfChannelsKey, AVSampleRateKey,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};
use std::path::Path;

// kAudioFormatMPEG4AAC, the four-char code 'aac '.
const FORMAT_AAC: isize = 0x6161_6320;
// Ample for speech recognition: an hour of audio comes to about 30 MB.
const BIT_RATE: isize = 64_000;
const FRAMES_PER_READ: u32 = 1 << 16;

fn file_url(path: &Path) -> objc2::rc::Retained<NSURL> {
    NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()))
}

/// Keeps the source sample rate and channel count: the writer only accepts
/// buffers in its own processing format.
pub fn encode_m4a(wav: &Path, out: &Path) -> Result<(), String> {
    unsafe {
        let input = AVAudioFile::initForReading_error(
            AVAudioFile::alloc(),
            &file_url(wav),
        )
        .map_err(|e| format!("AAC read open: {e:?}"))?;
        let format = input.processingFormat();
        let (Some(id_key), Some(rate_key), Some(channels_key), Some(bit_key)) = (
            AVFormatIDKey,
            AVSampleRateKey,
            AVNumberOfChannelsKey,
            AVEncoderBitRateKey,
        ) else {
            return Err("AAC settings keys unavailable".to_string());
        };
        let id = NSNumber::new_isize(FORMAT_AAC);
        let rate = NSNumber::new_f64(format.sampleRate());
        let channels = NSNumber::new_isize(format.channelCount() as isize);
        let bits = NSNumber::new_isize(BIT_RATE);
        let values: [&AnyObject; 4] = [&id, &rate, &channels, &bits];
        let settings = NSDictionary::<NSString, AnyObject>::from_slices(
            &[id_key, rate_key, channels_key, bit_key],
            &values,
        );
        let output =
            AVAudioFile::initForWriting_settings_commonFormat_interleaved_error(
                AVAudioFile::alloc(),
                &file_url(out),
                &settings,
                AVAudioCommonFormat::PCMFormatFloat32,
                false,
            )
            .map_err(|e| format!("AAC write open: {e:?}"))?;
        let buffer = AVAudioPCMBuffer::initWithPCMFormat_frameCapacity(
            AVAudioPCMBuffer::alloc(),
            &format,
            FRAMES_PER_READ,
        )
        .ok_or("AAC buffer")?;
        while input.framePosition() < input.length() {
            input
                .readIntoBuffer_error(&buffer)
                .map_err(|e| format!("AAC read: {e:?}"))?;
            if buffer.frameLength() == 0 {
                break;
            }
            output
                .writeFromBuffer_error(&buffer)
                .map_err(|e| format!("AAC write: {e:?}"))?;
        }
        // Dropping the writer finalizes the file before anyone reads it.
        drop(output);
        Ok(())
    }
}

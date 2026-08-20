async function signOut(): Promise<void> {
  const me = await fetch("/v1/auth/me").then((r) =>
    r.ok ? (r.json() as Promise<{ csrf: string }>) : null,
  );
  if (me?.csrf) {
    await fetch("/v1/auth/logout", {
      method: "POST",
      headers: { "x-csrf-token": me.csrf },
    });
  }
  const fr = location.pathname.startsWith("/fr");
  location.href = fr ? "/fr/login" : "/login";
}

document.getElementById("sign-out")?.addEventListener("click", () => {
  void signOut();
});

export {};

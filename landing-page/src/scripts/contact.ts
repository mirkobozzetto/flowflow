// Booking links carry the address reversed and base64-encoded
// (lib/links.ts): it only becomes a mailto when someone clicks.
function decode(code: string): string {
  return [...atob(code)].reverse().join("");
}

addEventListener("click", (event) => {
  const link = (event.target as Element | null)?.closest<HTMLElement>(
    "[data-contact]",
  );
  const code = link?.dataset.contact;
  if (!code) return;
  event.preventDefault();
  location.href = decode(code);
});

export {};

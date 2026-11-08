const status = document.getElementById("copy-status");

for (const button of document.querySelectorAll<HTMLButtonElement>(
  "[data-copy]",
)) {
  button.addEventListener("click", async () => {
    const source = document.getElementById(button.dataset.copy ?? "");
    if (!source || !status) return;

    try {
      await navigator.clipboard.writeText(source.textContent ?? "");
      button.textContent = button.dataset.done ?? "";
      status.textContent = status.dataset.done ?? "";
    } catch {
      status.textContent = status.dataset.failed ?? "";
      const range = document.createRange();
      range.selectNodeContents(source);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    }
  });
}

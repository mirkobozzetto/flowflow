const copyStatus = document.getElementById("copy-status");

for (const button of document.querySelectorAll<HTMLButtonElement>(
  "[data-copy]",
)) {
  button.addEventListener("click", async () => {
    const source = document.getElementById(button.dataset.copy ?? "");
    if (!source || !copyStatus) return;

    try {
      await navigator.clipboard.writeText(source.textContent ?? "");
      button.textContent = button.dataset.done ?? "";
      copyStatus.textContent = copyStatus.dataset.done ?? "";
    } catch {
      copyStatus.textContent = copyStatus.dataset.failed ?? "";
      const range = document.createRange();
      range.selectNodeContents(source);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    }
  });
}

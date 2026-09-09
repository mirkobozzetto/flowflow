document.querySelectorAll<HTMLButtonElement>("[data-copy]").forEach((button) => {
  button.addEventListener("click", async () => {
    const source = document.getElementById(button.dataset.copy ?? "");
    const status = document.getElementById("copy-status");
    if (!source || !status) return;

    try {
      await navigator.clipboard.writeText(source.textContent ?? "");
      button.textContent = "Copié";
      status.textContent = "Texte copié. Aucun jeton personnel dans cet exemple.";
    } catch {
      status.textContent = "Copie indisponible. Sélectionnez et copiez le texte manuellement.";
      const range = document.createRange();
      range.selectNodeContents(source);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    }
  });
});

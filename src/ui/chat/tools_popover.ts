// The existing + wrappers anchor the popover on both platforms. Only menu DOM
// is managed here; the composer and its keyboard handling remain untouched.
(() => {
    const menu = document.querySelector<HTMLElement>('.tools-popover');
    const trigger = menu?.parentElement?.querySelector<HTMLButtonElement>(':scope > button');
    const root = menu?.querySelector<HTMLElement>('.tools-root');
    const sub = menu?.querySelector<HTMLElement>('.tools-sub');
    if (!menu || !trigger || !root || !sub) return;
    let returnTo: HTMLElement = root.querySelector('button')!;
    const focus = (element: HTMLElement | null) => element?.focus({ preventScroll: true });
    const fit = () => {
        const top = window.visualViewport?.offsetTop ?? 0;
        menu.style.setProperty('--tools-space', `${Math.max(0, trigger.getBoundingClientRect().top - top - 16)}px`);
    };
    const remember = (event: MouseEvent) => {
        const row = (event.target as Element).closest<HTMLElement>('[data-pane]');
        if (row) returnTo = row;
    };
    root.addEventListener('click', remember);
    const panes = new MutationObserver(() => {
        focus(root.getAttribute('data-away') === 'true' ? sub.querySelector('button') : returnTo);
    });
    panes.observe(root, { attributes: true, attributeFilter: ['data-away'] });
    window.visualViewport?.addEventListener('resize', fit);
    window.visualViewport?.addEventListener('scroll', fit);
    window.addEventListener('resize', fit);
    fit();
    focus(returnTo);
    // Dioxus unmounts on outside tap, Escape, selection or navigation. Release
    // listeners in every case; do not steal focus from the next screen/input.
    const removal = new MutationObserver(() => {
        if (menu.isConnected) return;
        panes.disconnect();
        removal.disconnect();
        root.removeEventListener('click', remember);
        window.visualViewport?.removeEventListener('resize', fit);
        window.visualViewport?.removeEventListener('scroll', fit);
        window.removeEventListener('resize', fit);
        if (trigger.isConnected && document.activeElement === document.body) focus(trigger);
    });
    removal.observe(document.body, { childList: true, subtree: true });
})();

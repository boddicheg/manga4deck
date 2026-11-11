const root = document.querySelector(".app");
if (root) await new Promise((resolve) => {
    let previous = null;
    const update = () => {
        if (!root.isConnected) {
            resize.disconnect();
            content.disconnect();
            resolve();
            return;
        }
        const grid = root.querySelector(".series-grid");
        if (!grid) return;
        const columns = getComputedStyle(grid).gridTemplateColumns.split(/\s+/).filter(Boolean).length;
        if (columns > 0 && columns !== previous) {
            previous = columns;
            dioxus.send(columns);
        }
    };
    const resize = new ResizeObserver(update);
    const content = new MutationObserver(update);
    resize.observe(root);
    content.observe(document.body, { childList: true, subtree: true });
    update();
});

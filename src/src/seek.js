const target = await dioxus.recv();
const reader = document.querySelector(".reader-page");
if (reader) await new Promise((resolve) => {
    const generation = Number(reader.dataset.seekGeneration || 0) + 1;
    reader.dataset.seekGeneration = generation;
    let observer;
    let timeout;
    const stop = () => {
        observer?.disconnect();
        clearTimeout(timeout);
        resolve();
    };
    const findPage = () => {
        if (!reader.isConnected || Number(reader.dataset.seekGeneration) !== generation) {
            stop();
            return;
        }
        const page = reader.querySelector(`[data-reader-page="${target}"]`);
        if (!page) return;
        stop();
        requestAnimationFrame(() => {
            if (page.isConnected && Number(reader.dataset.seekGeneration) === generation) {
                page.scrollIntoView({ block: "start", behavior: "smooth" });
            }
        });
    };
    observer = new MutationObserver(findPage);
    observer.observe(reader, { childList: true, subtree: true });
    timeout = setTimeout(stop, 2000);
    findPage();
});

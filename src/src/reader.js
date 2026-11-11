// Runs for the lifetime of one reader element. Downloading an image is never
// evidence that it was read: report only a successfully loaded, visible page.
const reader = document.querySelector(".reader-page");
if (reader) await new Promise((resolve) => {
    let lastPage = null;
    let timer;
    let stopped = false;
    const report = () => {
        if (!reader.isConnected) return stop();
        const bounds = reader.getBoundingClientRect();
        const bar = reader.querySelector(".reader-bar");
        const top = Math.max(bounds.top, bar?.getBoundingClientRect().bottom ?? bounds.top);
        const bottom = Math.min(bounds.bottom, window.innerHeight);
        if (bottom <= top) return;
        const readingLine = top + Math.min(120, (bottom - top) / 4);
        let candidate = null;
        let distance = Infinity;
        for (const image of reader.querySelectorAll(".reader-image")) {
            const rect = image.getBoundingClientRect();
            if (rect.bottom <= top || rect.top >= bottom) continue;
            const gap = Math.max(rect.top - readingLine, readingLine - rect.bottom, 0);
            if (gap < distance) {
                candidate = image;
                distance = gap;
            }
        }
        // A later image may finish first while the reading position is still
        // over an unloaded placeholder. Do not jump past that placeholder.
        if (candidate !== null && candidate.complete && candidate.naturalWidth > 0) {
            const page = Number(candidate.closest("[data-reader-page]").dataset.readerPage);
            if (page !== lastPage) {
                lastPage = page;
                dioxus.send(page);
            }
        }
    };
    // Wait for scrolling/layout to settle so a slider jump does not record every
    // intermediate page it passes during smooth scrolling.
    const schedule = () => {
        clearTimeout(timer);
        timer = setTimeout(report, 150);
    };
    const resize = new ResizeObserver(schedule);
    const observeImages = () => {
        for (const image of reader.querySelectorAll(".reader-image")) resize.observe(image);
        schedule();
    };
    const content = new MutationObserver(observeImages);
    const removal = new MutationObserver(() => {
        if (!reader.isConnected) stop();
    });
    function stop() {
        if (stopped) return;
        stopped = true;
        clearTimeout(timer);
        reader.removeEventListener("scroll", schedule);
        reader.removeEventListener("load", schedule, true);
        window.removeEventListener("resize", schedule);
        resize.disconnect();
        content.disconnect();
        removal.disconnect();
        resolve();
    }
    reader.addEventListener("scroll", schedule, { passive: true });
    reader.addEventListener("load", schedule, true);
    window.addEventListener("resize", schedule);
    resize.observe(reader);
    content.observe(reader, { childList: true, subtree: true });
    removal.observe(document.body, { childList: true, subtree: true });
    observeImages();
});

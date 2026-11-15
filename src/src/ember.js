// Keyboard and standard-mapped gamepad navigation for the EMBER screens.
// Native inputs keep their text-editing keys; navigation uses the rendered layout.
const root = document.querySelector('.ember-screen');
if (root) await new Promise(resolve => {
    let frame = 0;
    let stopped = false;
    let previousView = '';
    let previousDialog = null;
    let returnFocus = null;
    let lastFolder = null;
    let lastSeries = null;
    let previousPadKeys = new Set();
    const repeats = new Map();
    const visible = el => !el.disabled && el.getClientRects().length > 0;
    const scope = () => root.querySelector('.ember-dialog') || root;
    const controls = () => [...scope().querySelectorAll('button, input')].filter(visible);
    const focus = el => {
        if (!el || !visible(el)) return;
        el.focus({ preventScroll: true });
        const carousel = el.closest('.ember-library-carousel');
        if (carousel) {
            const box = el.getBoundingClientRect(), viewport = carousel.getBoundingClientRect();
            let left = carousel.scrollLeft;
            if (box.left < viewport.left + 8) left += box.left - viewport.left - 8;
            if (box.right > viewport.right - 8) left += box.right - viewport.right + 8;
            carousel.scrollTo({ left, behavior: 'instant' });
            // Reveal the row vertically without scrolling the whole page sideways.
            carousel.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' });
        } else el.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' });
    };
    const preferred = () => scope().querySelector('.ember-series-card[data-autofocus="true"]') || scope().querySelector('.ember-folder[data-autofocus="true"]') || scope().querySelector('[data-autofocus="true"]') || controls()[0];
    const onFocus = () => {
        const folder = document.activeElement?.closest('.ember-folder');
        if (folder) lastFolder = folder;
        root.dataset.folderFocus = String(!!folder);
        const card = document.activeElement?.closest('.ember-series-card');
        if (card) lastSeries = card;
        root.dataset.libraryFocus = card ? 'series' : 'filters';
    };
    const updateArrows = () => {
        const rail = root.querySelector('.ember-library-carousel');
        if (!rail) return;
        root.querySelector('[data-library-scroll="previous"]')?.setAttribute('aria-disabled', String(rail.scrollLeft <= 1));
        root.querySelector('[data-library-scroll="next"]')?.setAttribute('aria-disabled', String(rail.scrollLeft + rail.clientWidth >= rail.scrollWidth - 1));
    };
    const onCarouselClick = event => {
        const button = event.target.closest('[data-library-scroll]');
        const rail = root.querySelector('.ember-library-carousel');
        if (!button || !rail || button.disabled || button.getAttribute('aria-disabled') === 'true') return;
        rail.scrollBy({ left: (button.dataset.libraryScroll === 'previous' ? -1 : 1) * rail.clientWidth, behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth' });
    };
    const updateTitles = () => {
        for (const title of root.querySelectorAll('.ember-series-title')) {
            const text = title.querySelector('.ember-series-title-text');
            const track = title.querySelector('.ember-series-title-track');
            if (!text || !track) continue;
            const width = text.getBoundingClientRect().width;
            title.dataset.overflow = String(width > title.clientWidth + 1);
            track.style.setProperty('--ember-title-offset', `${-(width + 32)}px`);
            track.style.setProperty('--ember-title-duration', `${Math.max(10, (width + 32) / 32)}s`);
        }
    };
    const updateFocus = () => {
        if (!root.isConnected) { cleanup(); return; }
        if (root.getAttribute('aria-busy') === 'true') return;
        const dialog = root.querySelector('.ember-dialog');
        const view = root.dataset.emberView;
        if (dialog !== previousDialog) {
            if (dialog) {
                returnFocus = document.activeElement;
                focus(dialog.querySelector('[data-autofocus="true"]'));
            } else if (returnFocus?.isConnected) {
                focus(returnFocus);
                returnFocus = null;
            }
            previousDialog = dialog;
        }
        if (view !== previousView || !controls().includes(document.activeElement)) {
            focus(preferred());
        }
        previousView = view;
        updateTitles();
        updateArrows();
    };
    const move = key => {
        const current = document.activeElement;
        if (!controls().includes(current)) { updateFocus(); return; }
        const folder = current.closest('.ember-folder');
        const home = root.dataset.emberView === 'Home';
        const library = root.dataset.emberView === 'Your library';
        const seriesCard = current.closest('.ember-series-card');
        const filter = current.closest('.ember-series-filter');
        const activeFilter = root.querySelector('.ember-series-filter[aria-pressed="true"]');
        const chosenSeries = lastSeries?.isConnected ? lastSeries : root.querySelector('.ember-series-card[data-autofocus="true"], .ember-series-card');
        if (library) {
            if (seriesCard) {
                if (key === 'ArrowLeft' || key === 'ArrowRight') {
                    const list = [...seriesCard.parentElement.querySelectorAll('.ember-series-card')];
                    focus(list[list.indexOf(seriesCard) + (key === 'ArrowLeft' ? -1 : 1)]);
                } else if (key === 'ArrowUp') focus(activeFilter);
                return;
            }
            if (filter) {
                if (key === 'ArrowLeft' || key === 'ArrowRight') {
                    const list = [...root.querySelectorAll('.ember-series-filter')];
                    focus(list[list.indexOf(filter) + (key === 'ArrowLeft' ? -1 : 1)]);
                } else if (key === 'ArrowUp') focus(root.querySelector('[data-nav="1"]'));
                else if (key === 'ArrowDown') focus(chosenSeries || root.querySelector('.ember-series-library .ember-empty button'));
                return;
            }
            if (current.closest('.ember-nav') && key === 'ArrowDown') { focus(activeFilter); return; }
        }
        const reading = root.querySelector('.ember-resume button, .ember-empty button');
        const chosenFolder = lastFolder?.isConnected ? lastFolder : root.querySelector('.ember-folder[data-autofocus="true"]');
        if (folder) {
            if (key === 'ArrowLeft' || key === 'ArrowRight') {
                const list = [...folder.parentElement.querySelectorAll('.ember-folder')];
                focus(list[list.indexOf(folder) + (key === 'ArrowLeft' ? -1 : 1)]);
                return;
            }
            if (key === 'ArrowUp') { focus(reading || root.querySelector('[data-nav="0"]')); return; }
            if (key === 'ArrowDown') { focus(root.querySelector('.ember-library-error button')); return; }
        }
        if (home && current === reading && key === 'ArrowDown' && chosenFolder) { focus(chosenFolder); return; }
        if (home && current === reading && key === 'ArrowUp') { focus(root.querySelector('[data-nav="0"]')); return; }
        if (home && current.closest('.ember-nav') && key === 'ArrowDown') { focus(reading || chosenFolder); return; }
        const rect = current.getBoundingClientRect();
        const x = rect.x + rect.width / 2, y = rect.y + rect.height / 2;
        const horizontal = key === 'ArrowLeft' || key === 'ArrowRight';
        const sign = key === 'ArrowLeft' || key === 'ArrowUp' ? -1 : 1;
        const next = controls().filter(el => {
            if (el === current) return false;
            const rail = el.closest('.ember-library-carousel');
            if (!rail) return true;
            const box = el.getBoundingClientRect(), viewport = rail.getBoundingClientRect();
            return box.right > viewport.left && box.left < viewport.right;
        }).map(el => {
            const box = el.getBoundingClientRect();
            const dx = box.x + box.width / 2 - x, dy = box.y + box.height / 2 - y;
            const primary = horizontal ? dx * sign : dy * sign;
            const secondary = Math.abs(horizontal ? dy : dx);
            return { el, primary, score: primary + secondary * 4 };
        }).filter(item => item.primary > 8).sort((a, b) => a.score - b.score)[0];
        focus(next?.el);
    };
    const onKey = event => {
        if (!root.isConnected) return;
        if (event.ctrlKey || event.metaKey || event.altKey || event.isComposing) return;
        const editing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName);
        const key = event.key;
        if (editing && !event.emberGamepad && key !== 'Escape' && key !== 'Tab') return;
        if (root.getAttribute('aria-busy') === 'true') {
            if (['Enter', ' ', 'Escape', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(key)) event.preventDefault();
            event.stopPropagation();
            return;
        }
        if (!controls().includes(document.activeElement)) updateFocus();
        const dialog = root.querySelector('.ember-dialog');
        const libraryCard = root.dataset.emberView === 'Your library' && document.activeElement?.closest('.ember-series-card');
        // Preserve the app's existing F2 download implementation and its status updates.
        if (!dialog && libraryCard && key === 'F2') return;
        if (!dialog && libraryCard && key.toLowerCase() === 'x') {
            document.activeElement.dispatchEvent(new KeyboardEvent('keydown', { key: 'F2', code: 'F2', bubbles: true, cancelable: true }));
            event.preventDefault(); event.stopPropagation(); return;
        }
        if (key === 'Tab' && dialog) {
            const list = controls();
            const index = list.indexOf(document.activeElement);
            focus(list[(index + (event.shiftKey ? -1 : 1) + list.length) % list.length]);
        } else if (key.startsWith('Arrow')) {
            move(key);
        } else if (key === 'Enter' || key === ' ' || key.toLowerCase() === 'a') {
            if (editing) document.activeElement.form?.requestSubmit();
            else document.activeElement?.click();
        } else if (key === 'Escape' || key === 'Backspace' || key.toLowerCase() === 'b') {
            const back = scope().querySelector('[data-back="true"]');
            if (back) back.click();
            else if (root.dataset.emberView === 'Your library') {
                if (libraryCard) focus(root.querySelector('.ember-series-filter[aria-pressed="true"]'));
                else root.querySelector('.ember-series-back')?.click();
            }
            else if (document.activeElement?.closest('.ember-folder')) focus(root.querySelector('.ember-resume button, .ember-empty button') || root.querySelector('[data-nav="0"]'));
            else if (root.dataset.emberView === 'Home') focus(root.querySelector('[data-nav="0"]'));
            else root.querySelector('[data-nav="0"]')?.click();
        } else if (!dialog && (key.toLowerCase() === 'x' || key === 'F2')) {
            root.querySelector('[data-nav="2"]')?.click();
        } else if (!dialog && key.toLowerCase() === 's') {
            root.querySelector('[data-nav="3"]')?.click();
        } else if (!dialog && key.toLowerCase() === 'y') {
            root.querySelector('[data-connection="true"]')?.click();
        } else return;
        event.preventDefault();
        event.stopPropagation();
    };
    const poll = now => {
        if (stopped) return;
        if (!document.hidden && document.hasFocus()) {
            const pressed = new Set();
            for (const pad of navigator.getGamepads?.() || []) {
                if (!pad || pad.mapping !== 'standard') continue;
                const mapping = { 0: 'Enter', 1: 'Escape', 2: 'x', 3: 'y', 12: 'ArrowUp', 13: 'ArrowDown', 14: 'ArrowLeft', 15: 'ArrowRight' };
                for (const [button, key] of Object.entries(mapping)) if (pad.buttons[button]?.pressed) pressed.add(key);
                if (pad.axes[0] < -.55) pressed.add('ArrowLeft');
                if (pad.axes[0] > .55) pressed.add('ArrowRight');
                if (pad.axes[1] < -.55) pressed.add('ArrowUp');
                if (pad.axes[1] > .55) pressed.add('ArrowDown');
            }
            for (const key of pressed) {
                if (!previousPadKeys.has(key) || (key.startsWith('Arrow') && now >= (repeats.get(key) || 0))) {
                    const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
                    Object.defineProperty(event, 'emberGamepad', { value: true });
                    (root.contains(document.activeElement) ? document.activeElement : root).dispatchEvent(event);
                    repeats.set(key, now + (previousPadKeys.has(key) ? 150 : 400));
                }
            }
            previousPadKeys = pressed;
        } else {
            previousPadKeys = new Set();
            repeats.clear();
        }
        frame = requestAnimationFrame(poll);
    };
    const observer = new MutationObserver(updateFocus);
    function cleanup() {
        if (stopped) return;
        stopped = true;
        cancelAnimationFrame(frame);
        observer.disconnect();
        resize.disconnect();
        document.removeEventListener('keydown', onKey, true);
        root.removeEventListener('focusin', onFocus);
        root.removeEventListener('click', onCarouselClick);
        root.removeEventListener('scroll', updateArrows, true);
        resolve();
    }
    document.addEventListener('keydown', onKey, true);
    root.addEventListener('focusin', onFocus);
    root.addEventListener('click', onCarouselClick);
    root.addEventListener('scroll', updateArrows, true);
    const resize = new ResizeObserver(() => {
        updateTitles();
        if (document.activeElement?.matches('.ember-folder, .ember-series-card')) focus(document.activeElement);
        updateArrows();
    });
    resize.observe(root);
    observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['aria-busy', 'data-ember-view', 'disabled'] });
    updateFocus();
    document.fonts.ready.then(() => { if (root.isConnected) updateTitles(); });
    frame = requestAnimationFrame(poll);
});

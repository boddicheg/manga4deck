// Keyboard and standard-mapped gamepad navigation for the EMBER screens.
// Native inputs keep their text-editing keys; navigation uses the rendered layout.
const root = document.querySelector('.ember-screen');
if (root) await new Promise(resolve => {
    let frame = 0;
    let stopped = false;
    let previousView = '';
    let previousDialog = null;
    let returnFocus = null;
    let previousPadKeys = new Set();
    const repeats = new Map();
    const visible = el => !el.disabled && el.getClientRects().length > 0;
    const scope = () => root.querySelector('.ember-dialog') || root;
    const controls = () => [...scope().querySelectorAll('button, input')].filter(visible);
    const focus = el => {
        if (!el || !visible(el)) return;
        el.focus({ preventScroll: true });
        el.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' });
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
            focus(scope().querySelector('[data-autofocus="true"]') || controls()[0]);
        }
        previousView = view;
    };
    const move = key => {
        const current = document.activeElement;
        if (!controls().includes(current)) { updateFocus(); return; }
        const rect = current.getBoundingClientRect();
        const x = rect.x + rect.width / 2, y = rect.y + rect.height / 2;
        const horizontal = key === 'ArrowLeft' || key === 'ArrowRight';
        const sign = key === 'ArrowLeft' || key === 'ArrowUp' ? -1 : 1;
        const next = controls().filter(el => el !== current).map(el => {
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
        document.removeEventListener('keydown', onKey, true);
        resolve();
    }
    document.addEventListener('keydown', onKey, true);
    observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['aria-busy', 'data-ember-view', 'disabled'] });
    updateFocus();
    frame = requestAnimationFrame(poll);
});

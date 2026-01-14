import { formatDate, formatTime, formatDateTime } from '../utils/datetime.js';

// Timeline component - infinite scroll timeline visualization
export default () => ({
    virtualWidth: 1000000,
    minDurationMs: 60 * 60 * 1000,
    maxDurationMs: 14 * 24 * 60 * 60 * 1000,

    viewDurationMs: 60 * 60 * 1000,
    viewStart: null,
    viewEnd: null,
    anchorTime: null,
    viewportWidth: 1,
    initialScrollLeft: 0,
    msPerPixel: 0,

    markers: [],
    nowPx: 0,
    nowVisible: false,

    rows: [],  // Array of { type: 'schedules'|'events', label: string, items: [] }

    isDragging: false,
    dragPointerId: null,
    dragStartX: 0,
    dragStartViewStartMs: 0,
    dragStartScrollLeft: 0,
    dragNeedsNotify: false,
    isPointerCaptured: false,

    pointerInside: false,
    pointerViewportX: null,
    pointerContentX: null,
    hoverTime: null,

    ignoreScroll: false,
    scrollRaf: null,

    get zoomLabel() {
        const hours = this.viewDurationMs / (1000 * 60 * 60);
        return hours >= 24 ? `${Math.round(hours / 24 * 10) / 10}d` : `${Math.round(hours)}h`;
    },

    get startInputValue() {
        return this.viewStart ? this.toLocalDatetimeString(this.viewStart) : '';
    },

    get endInputValue() {
        return this.viewEnd ? this.toLocalDatetimeString(this.viewEnd) : '';
    },

    get hoverLabel() {
        if (!this.hoverTime) return '';
        return formatDateTime(this.hoverTime);
    },

    init() {
        this.$nextTick(() => {
            const container = this.$refs.scrollContainer;
            const content = this.$refs.content;
            if (!container || !content) return;

            content.style.width = `${this.virtualWidth}px`;
            this.updateViewportMetrics();

            const now = new Date();
            this.viewStart = new Date(now.getTime() - this.viewDurationMs / 2);
            this.viewEnd = new Date(this.viewStart.getTime() + this.viewDurationMs);
            this.anchorTime = new Date(this.viewStart.getTime() - this.initialScrollLeft * this.msPerPixel);

            this.setScrollLeft(this.initialScrollLeft, true);
            this.render();
            this.notifyViewChange();

            // Watch auth changes
            this.$watch('$store.auth.key', () => {
                Alpine.store('schedules').loadedRange = null;
                Alpine.store('schedules').fetch(true);
                if (Alpine.store('predictions')) {
                    Alpine.store('predictions').loadedRange = null;
                    Alpine.store('predictions').fetch(true);
                }
            });

            // Watch for changes in schedules and predictions to rebuild rows
            this.$watch('$store.schedules.items', () => this.buildRows());
            if (Alpine.store('predictions')) {
                this.$watch('$store.predictions.items', () => this.buildRows());
            }
            if (Alpine.store('events')) {
                this.$watch('$store.events.items', () => this.buildRows());
            }

            // Initial row build
            this.buildRows();
        });
    },

    buildRows() {
        const rows = [];

        // Schedules row (always first)
        rows.push({
            type: 'schedules',
            label: 'Schedules',
            items: Alpine.store('schedules').items || [],
        });

        const eventRows = [];
        if (Alpine.store('events')) {
            eventRows.push(...this.buildEventRowsFromStore(Alpine.store('events')));
        }
        if (Alpine.store('predictions')) {
            eventRows.push(...this.buildPredictionRows());
        }

        this.rows = rows.concat(eventRows);
    },

    buildPredictionRows() {
        const store = Alpine.store('predictions');
        const grouped = store.passesBySatellite();
        const rows = [];
        for (const [satellite, passes] of Object.entries(grouped)) {
            rows.push({
                type: 'events',
                label: satellite,
                items: passes,
                source: 'prediction',
            });
        }
        return rows;
    },

    buildEventRowsFromStore(store) {
        const events = store.visibleItems ? store.visibleItems() : (store.items || []);
        const grouped = new Map();
        for (const event of events) {
            const label = this.eventSourceLabel(event);
            if (!grouped.has(label)) grouped.set(label, []);
            grouped.get(label).push(event);
        }

        const rows = [];
        for (const [label, items] of grouped.entries()) {
            rows.push({
                type: 'events',
                label,
                items,
                source: 'event',
            });
        }
        return rows;
    },

    updateViewportMetrics() {
        const container = this.$refs.scrollContainer;
        this.viewportWidth = Math.max(container?.clientWidth || 1, 1);
        this.initialScrollLeft = Math.max(0, (this.virtualWidth - this.viewportWidth) / 2);
        this.msPerPixel = this.viewDurationMs / this.viewportWidth;
    },

    setScrollLeft(value, skipUpdate = false) {
        const container = this.$refs.scrollContainer;
        this.ignoreScroll = true;
        container.scrollLeft = value;
        requestAnimationFrame(() => {
            this.ignoreScroll = false;
            if (!skipUpdate) this.updateViewFromScroll();
        });
    },

    onScroll() {
        if (this.isDragging) return;
        if (this.pointerInside && typeof this.pointerViewportX === 'number') {
            this.pointerContentX = this.$refs.scrollContainer.scrollLeft + this.pointerViewportX;
            this.updateHoverTime();
        }
        if (this.ignoreScroll || this.scrollRaf) return;
        this.scrollRaf = requestAnimationFrame(() => {
            this.scrollRaf = null;
            this.updateViewFromScroll();
        });
    },

    updateViewFromScroll() {
        const scrollLeft = this.$refs.scrollContainer.scrollLeft;
        const viewStartMs = this.anchorTime.getTime() + scrollLeft * this.msPerPixel;
        this.viewStart = new Date(viewStartMs);
        this.viewEnd = new Date(viewStartMs + this.viewDurationMs);
        this.render();
        this.notifyViewChange();
        if (!this.isDragging) this.recenterIfNeeded();
    },

    recenterIfNeeded() {
        const threshold = this.viewportWidth * 0.2;
        const maxScroll = this.virtualWidth - this.viewportWidth;
        const scrollLeft = this.$refs.scrollContainer.scrollLeft;
        if (scrollLeft < threshold || scrollLeft > Math.max(0, maxScroll - threshold)) {
            this.anchorTime = new Date(this.viewStart.getTime() - this.initialScrollLeft * this.msPerPixel);
            this.setScrollLeft(this.initialScrollLeft, true);
        }
    },

    reflow() {
        if (!this.$refs.scrollContainer || !this.viewStart) return;
        this.updateViewportMetrics();
        this.anchorTime = new Date(this.viewStart.getTime() - this.initialScrollLeft * this.msPerPixel);
        this.setScrollLeft(this.initialScrollLeft, true);
        this.render();
        this.notifyViewChange();
    },

    notifyViewChange() {
        if (!this.viewStart || !this.viewEnd) return;
        Alpine.store('schedules').setViewRange(this.viewStart.toISOString(), this.viewEnd.toISOString());
        if (Alpine.store('predictions')) {
            Alpine.store('predictions').setViewRange(this.viewStart.toISOString(), this.viewEnd.toISOString());
        }
        if (Alpine.store('events')) {
            Alpine.store('events').setViewRange(this.viewStart.toISOString(), this.viewEnd.toISOString());
        }
        this.buildRows();
    },

    zoomIn() { this.adjustZoom(0.5); },
    zoomOut() { this.adjustZoom(2); },

    adjustZoom(multiplier) {
        const proposed = this.viewDurationMs * multiplier;
        const newDuration = Math.min(Math.max(proposed, this.minDurationMs), this.maxDurationMs);
        if (newDuration === this.viewDurationMs) return;

        const focusPx = typeof this.pointerViewportX === 'number' ? this.pointerViewportX : this.viewportWidth / 2;
        const focusTime = this.getTimeAtViewportPx(focusPx);

        this.viewDurationMs = newDuration;
        this.msPerPixel = this.viewDurationMs / this.viewportWidth;

        const newStartMs = focusTime.getTime() - focusPx * this.msPerPixel;
        this.viewStart = new Date(newStartMs);
        this.viewEnd = new Date(newStartMs + this.viewDurationMs);
        this.anchorTime = new Date(this.viewStart.getTime() - this.$refs.scrollContainer.scrollLeft * this.msPerPixel);

        this.render();
        this.notifyViewChange();
    },

    goToNow() { this.centerOnTime(new Date()); },

    centerOnTime(centerTime) {
        const startMs = centerTime.getTime() - this.viewDurationMs / 2;
        this.viewStart = new Date(startMs);
        this.viewEnd = new Date(startMs + this.viewDurationMs);
        this.anchorTime = new Date(startMs - this.initialScrollLeft * this.msPerPixel);
        this.setScrollLeft(this.initialScrollLeft, true);
        this.render();
        this.notifyViewChange();
    },

    onStartInputChange(event) {
        this.onTimeRangeChange(event.target.value, this.endInputValue);
    },

    onEndInputChange(event) {
        this.onTimeRangeChange(this.startInputValue, event.target.value);
    },

    onTimeRangeChange(startVal, endVal) {
        if (!startVal || !endVal) return;
        const start = new Date(startVal);
        const end = new Date(endVal);
        if (isNaN(start.getTime()) || isNaN(end.getTime())) return;

        let duration = Math.max(this.minDurationMs, Math.min(this.maxDurationMs, end - start));
        this.viewDurationMs = duration;
        this.msPerPixel = duration / this.viewportWidth;
        this.centerOnTime(new Date(start.getTime() + duration / 2));
    },

    getTimeAtViewportPx(viewportPx) {
        const clamped = Math.max(0, Math.min(this.viewportWidth, viewportPx));
        return this.pixelToTime(this.$refs.scrollContainer.scrollLeft + clamped);
    },

    onPointerDown(event) {
        if (event.button !== 0 && event.pointerType === 'mouse') return;

        this.pointerInside = true;
        this.updatePointerPosition(event);
        this.dragPointerId = event.pointerId;
        this.dragStartX = event.clientX;
        this.dragStartViewStartMs = this.viewStart ? this.viewStart.getTime() : Date.now();
        this.dragStartScrollLeft = this.$refs.scrollContainer.scrollLeft;
        this.hasDragged = false;
        this.isDragging = false;
        this.dragNeedsNotify = false;
        this.isPointerCaptured = false;
    },

    onPointerMove(event) {
        this.updatePointerPosition(event);
        if (event.pointerId !== this.dragPointerId) return;

        const dragDistance = Math.abs(event.clientX - this.dragStartX);
        if (!this.isDragging && dragDistance > 3) {
            // Start dragging only after moving more than 3px (reduced threshold for better UX)
            this.isDragging = true;
            this.hasDragged = true;
            if (!this.isPointerCaptured) {
                this.$refs.scrollContainer.setPointerCapture(event.pointerId);
                this.isPointerCaptured = true;
            }
        }

        if (this.isDragging) {
            event.preventDefault();
            this.applyDrag(event.clientX);
        }
    },

    onPointerUp(event) {
        if (event.pointerId === this.dragPointerId) {
            this.isDragging = false;
            this.dragPointerId = null;
            this.hasDragged = false;
            this.dragStartViewStartMs = 0;
            this.dragStartScrollLeft = 0;
            if (this.dragNeedsNotify) {
                this.dragNeedsNotify = false;
                this.notifyViewChange();
            }
            if (this.isPointerCaptured) {
                this.$refs.scrollContainer.releasePointerCapture(event.pointerId);
                this.isPointerCaptured = false;
            }
        }
    },

    onPointerEnter(event) {
        this.pointerInside = true;
        this.updatePointerPosition(event);
    },

    onPointerLeave() {
        if (this.isDragging) return;
        this.pointerInside = false;
        this.pointerViewportX = null;
        this.pointerContentX = null;
        this.hoverTime = null;
    },

    updatePointerPosition(event) {
        const rect = this.$refs.scrollContainer.getBoundingClientRect();
        this.pointerViewportX = Math.max(0, Math.min(this.viewportWidth, event.clientX - rect.left));
        this.pointerContentX = this.$refs.scrollContainer.scrollLeft + this.pointerViewportX;
        this.pointerInside = true;
        this.updateHoverTime();
    },

    updateHoverTime() {
        this.hoverTime = typeof this.pointerContentX === 'number' ? this.pixelToTime(this.pointerContentX) : null;
    },

    applyDrag(clientX) {
        const deltaMs = (this.dragStartX - clientX) * this.msPerPixel;
        const newStartMs = this.dragStartViewStartMs + deltaMs;
        this.viewStart = new Date(newStartMs);
        this.viewEnd = new Date(newStartMs + this.viewDurationMs);
        this.anchorTime = new Date(newStartMs - this.dragStartScrollLeft * this.msPerPixel);
        this.render();
        this.dragNeedsNotify = true;
    },

    getItemStart(item) {
        return item.start ?? item.aos;
    },

    getItemEnd(item) {
        return item.end ?? item.los;
    },

    itemStyle(item) {
        const start = this.getItemStart(item);
        const end = this.getItemEnd(item);
        if (!start || !end) return 'display: none';
        const startPx = Math.max(0, this.timeToPx(new Date(start)));
        const endPx = Math.min(this.virtualWidth, this.timeToPx(new Date(end)));
        return `left: ${startPx}px; width: ${Math.max(4, endPx - startPx)}px`;
    },

    itemWidth(item) {
        const start = this.getItemStart(item);
        const end = this.getItemEnd(item);
        if (!start || !end) return 0;
        return this.timeToPx(new Date(end)) - this.timeToPx(new Date(start));
    },

    itemVisible(item) {
        const start = this.getItemStart(item);
        const end = this.getItemEnd(item);
        if (!start || !end) return false;
        if (!this.viewStart || !this.viewEnd) return false;
        const startDate = new Date(start);
        const endDate = new Date(end);
        return !(endDate <= this.viewStart || startDate >= this.viewEnd);
    },

    pixelToTime(px) {
        return this.anchorTime ? new Date(this.anchorTime.getTime() + px * this.msPerPixel) : new Date();
    },

    timeToPx(date) {
        return this.anchorTime ? (date.getTime() - this.anchorTime.getTime()) / this.msPerPixel : 0;
    },

    toLocalDatetimeString(date) {
        const pad = (n) => n.toString().padStart(2, '0');
        return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
    },

    render() {
        this.renderMarkers();
        this.updateNowMarker();
    },

    renderMarkers() {
        const hours = this.viewDurationMs / (1000 * 60 * 60);
        let intervalHours, majorInterval;

        if (hours <= 2) { intervalHours = 0.25; majorInterval = 1; }
        else if (hours <= 6) { intervalHours = 0.5; majorInterval = 2; }
        else if (hours <= 12) { intervalHours = 1; majorInterval = 6; }
        else if (hours <= 48) { intervalHours = 2; majorInterval = 12; }
        else if (hours <= 168) { intervalHours = 6; majorInterval = 24; }
        else { intervalHours = 24; majorInterval = 24 * 7; }

        const intervalMs = intervalHours * 60 * 60 * 1000;
        let current = new Date(Math.floor(this.viewStart.getTime() / intervalMs) * intervalMs);

        const newMarkers = [];
        while (current <= this.viewEnd) {
            const isMajor = intervalHours >= 1
                ? current.getHours() % majorInterval === 0 && current.getMinutes() === 0
                : current.getMinutes() === 0;
            const px = this.timeToPx(current);

            if (px >= 0 && px <= this.virtualWidth) {
                let label;
                if (isMajor && current.getHours() === 0 && current.getMinutes() === 0) {
                    label = formatDate(current);
                } else if (intervalHours < 1 || isMajor) {
                    label = formatTime(current);
                } else {
                    label = formatTime(current);
                }
                newMarkers.push({ time: current.getTime(), px, major: isMajor, label });
            }
            current = new Date(current.getTime() + intervalMs);
        }
        this.markers = newMarkers;
    },

    updateNowMarker() {
        const now = new Date();
        this.nowVisible = now >= this.viewStart && now <= this.viewEnd;
        if (this.nowVisible) this.nowPx = this.timeToPx(now);
    },

    scheduleStyle(schedule) {
        return this.itemStyle(schedule);
    },

    scheduleWidth(schedule) {
        return this.itemWidth(schedule);
    },

    scheduleVisible(schedule) {
        return this.itemVisible(schedule);
    },

    scheduleTitle(schedule) {
        return `${schedule.id}: ${formatTime(schedule.start)} - ${formatTime(schedule.end)}`;
    },

    // Pass helper methods
    passStyle(pass) {
        return this.itemStyle(pass);
    },

    passWidth(pass) {
        return this.itemWidth(pass);
    },

    passVisible(pass) {
        return this.itemVisible(pass);
    },

    passTitle(pass) {
        return `${pass.satellite}: Max ${pass.max_elevation_deg}° at ${formatTime(pass.tca)}`;
    },

    eventTitle(event) {
        if (event.satellite && event.tca) return this.passTitle(event);
        const start = this.getItemStart(event);
        const end = this.getItemEnd(event);
        const label = event.title || event.label || event.name || 'Event';
        if (!start || !end) return label;
        return `${label}: ${formatTime(start)} - ${formatTime(end)}`;
    },

    eventLabel(event) {
        if (typeof event.max_elevation_deg === 'number') {
            return `${Math.round(event.max_elevation_deg)}°`;
        }
        return event.short_label || event.label || event.title || '';
    },

    eventSourceLabel(event) {
        if (event?.source && typeof event.source === 'string') return event.source;
        if (event?.source && typeof event.source === 'object') {
            return event.source.name || event.source.label || event.source.id || 'Events';
        }
        return event.source_name || event.provider || event.origin || event.kind || event.type || 'Events';
    },

    openEventDetail(event) {
        if (Alpine.store('predictions') && event.satellite) {
            Alpine.store('predictions').openDetail(event);
            return;
        }
        const eventsStore = Alpine.store('events');
        if (eventsStore?.openDetail) {
            eventsStore.openDetail(event);
        } else if (eventsStore?.openDetailById && event.id) {
            eventsStore.openDetailById(event.id);
        }
    },
});

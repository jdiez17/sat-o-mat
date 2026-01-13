// Timeline visualization module
// Provides infinite-scroll style navigation and range-aware loading

const Timeline = {
    elements: {
        scrollContainer: null,
        content: null,
        markers: null,
        schedulesContainer: null,
        nowMarker: null,
        timeCursor: null,
        startInput: null,
        endInput: null,
        zoomLabel: null,
        hoverLabel: null,
    },
    schedules: [],
    virtualWidth: 1000000,
    viewportWidth: 0,
    msPerPixel: 0,
    viewDurationMs: 60 * 60 * 1000,
    minDurationMs: 60 * 60 * 1000,
    maxDurationMs: 14 * 24 * 60 * 60 * 1000,
    anchorTime: null,
    viewStart: null,
    viewEnd: null,
    initialScrollLeft: 0,
    fetchTimeout: null,
    fetchToken: 0,
    loadedRange: null,
    ignoreScroll: false,
    scrollRaf: null,
    isDragging: false,
    dragStartX: 0,
    dragStartScrollLeft: 0,
    dragPointerId: null,
    lastPointerViewportX: null,
    lastPointerContentX: null,
    pointerInside: false,

    init() {
        this.elements.scrollContainer = document.getElementById('timeline-scroll-container');
        this.elements.content = document.getElementById('timeline-content');
        this.elements.markers = document.getElementById('timeline-markers');
        this.elements.schedulesContainer = document.getElementById('timeline-schedules');
        this.elements.nowMarker = document.getElementById('timeline-now-marker');
        this.elements.timeCursor = document.getElementById('timeline-cursor');
        this.elements.startInput = document.getElementById('timeline-start');
        this.elements.endInput = document.getElementById('timeline-end');
        this.elements.zoomLabel = document.getElementById('timeline-zoom-level');
        this.elements.hoverLabel = document.getElementById('timeline-hover-label');

        if (!this.elements.scrollContainer || !this.elements.content) {
            return;
        }

        this.elements.content.style.width = `${this.virtualWidth}px`;
        this.updateViewportMetrics();

        const now = new Date();
        this.viewStart = new Date(now.getTime() - this.viewDurationMs / 2);
        this.viewEnd = new Date(this.viewStart.getTime() + this.viewDurationMs);
        this.anchorTime = new Date(this.viewStart.getTime() - this.initialScrollLeft * this.msPerPixel);
        this.setScrollLeft(this.initialScrollLeft, true);
        this.updateInputs();
        this.render();
        this.fetchSchedulesForView(true);
        this.notifyViewChange();

        this.bindEvents();
    },

    bindEvents() {
        this.elements.scrollContainer.addEventListener('scroll', () => this.onScroll());
        window.addEventListener('resize', () => this.onResize());

        this.elements.startInput?.addEventListener('change', () => this.onTimeRangeChange());
        this.elements.endInput?.addEventListener('change', () => this.onTimeRangeChange());

        document.getElementById('timeline-zoom-in')?.addEventListener('click', () => this.zoomIn());
        document.getElementById('timeline-zoom-out')?.addEventListener('click', () => this.zoomOut());
        document.getElementById('timeline-now-btn')?.addEventListener('click', () => this.goToNow());

        this.elements.scrollContainer.addEventListener('pointerdown', (event) =>
            this.onPointerDown(event),
        );
        this.elements.scrollContainer.addEventListener('pointermove', (event) =>
            this.onPointerMove(event),
        );
        this.elements.scrollContainer.addEventListener('pointerup', (event) =>
            this.onPointerUp(event),
        );
        this.elements.scrollContainer.addEventListener('pointercancel', (event) =>
            this.onPointerUp(event),
        );
        this.elements.scrollContainer.addEventListener('pointerenter', (event) =>
            this.onPointerEnter(event),
        );
        this.elements.scrollContainer.addEventListener('pointerleave', (event) =>
            this.onPointerLeave(event),
        );

        document.body.addEventListener('scheduleUpdated', () => {
            this.loadedRange = null;
            this.fetchSchedulesForView(true);
        });
        document.body.addEventListener('authChanged', () => {
            this.loadedRange = null;
            this.schedules = [];
            this.render();
            this.fetchSchedulesForView(true);
        });
    },

    onPointerDown(event) {
        if (event.button !== 0 && event.pointerType === 'mouse') {
            return;
        }
        this.pointerInside = true;
        this.updatePointerPosition(event);
        this.isDragging = true;
        this.dragPointerId = event.pointerId;
        this.dragStartX = event.clientX;
        this.dragStartScrollLeft = this.elements.scrollContainer.scrollLeft;
        this.elements.scrollContainer.setPointerCapture(event.pointerId);
        this.elements.scrollContainer.classList.add('cursor-grabbing');
        this.elements.content?.classList.add('select-none');
    },

    onPointerMove(event) {
        this.updatePointerPosition(event);
        if (!this.isDragging || event.pointerId !== this.dragPointerId) {
            return;
        }
        event.preventDefault();
        const delta = event.clientX - this.dragStartX;
        this.elements.scrollContainer.scrollLeft = this.dragStartScrollLeft - delta;
    },

    onPointerUp(event) {
        if (event.pointerId === this.dragPointerId) {
            this.isDragging = false;
            this.dragPointerId = null;
            this.elements.scrollContainer.releasePointerCapture(event.pointerId);
            this.elements.scrollContainer.classList.remove('cursor-grabbing');
            this.elements.content?.classList.remove('select-none');
        }
        this.updatePointerPosition(event);
    },

    onPointerEnter(event) {
        this.pointerInside = true;
        this.updatePointerPosition(event);
    },

    onPointerLeave() {
        if (this.isDragging) {
            return;
        }
        this.pointerInside = false;
        this.lastPointerViewportX = null;
        this.lastPointerContentX = null;
        this.updateTimeCursor();
    },

    updatePointerPosition(event) {
        if (!this.elements.scrollContainer) {
            return;
        }
        const rect = this.elements.scrollContainer.getBoundingClientRect();
        const viewportX = event.clientX - rect.left;
        this.lastPointerViewportX = Math.max(0, Math.min(this.viewportWidth, viewportX));
        this.lastPointerContentX =
            this.elements.scrollContainer.scrollLeft + this.lastPointerViewportX;
        if (!this.pointerInside) {
            this.pointerInside = true;
        }
        this.updateTimeCursor();
    },

    updateTimeCursor() {
        const cursor = this.elements.timeCursor;
        if (!cursor) {
            return;
        }
        if (
            !this.pointerInside ||
            typeof this.lastPointerContentX !== 'number' ||
            typeof this.lastPointerViewportX !== 'number'
        ) {
            cursor.classList.add('hidden');
            this.updateHoverLabel(null);
            return;
        }
        cursor.classList.remove('hidden');
        cursor.style.left = `${this.lastPointerContentX}px`;
        const hoverTime = this.pixelToTime(this.lastPointerContentX);
        this.updateHoverLabel(hoverTime);
    },

    updateViewportMetrics() {
        this.viewportWidth = Math.max(this.elements.scrollContainer.clientWidth || 0, 1);
        this.initialScrollLeft = Math.max(0, (this.virtualWidth - this.viewportWidth) / 2);
        this.msPerPixel = this.viewDurationMs / this.viewportWidth;
    },

    onResize() {
        const center = this.getCenterTime();
        this.updateViewportMetrics();
        this.anchorTime = new Date(
            this.viewStart.getTime() - this.initialScrollLeft * this.msPerPixel,
        );
        this.centerOnTime(center);
    },

    onScroll() {
        if (this.pointerInside && typeof this.lastPointerViewportX === 'number') {
            this.lastPointerContentX =
                this.elements.scrollContainer.scrollLeft + this.lastPointerViewportX;
            this.updateTimeCursor();
        }
        if (this.ignoreScroll) {
            return;
        }
        if (this.scrollRaf) {
            return;
        }
        this.scrollRaf = requestAnimationFrame(() => {
            this.scrollRaf = null;
            this.updateViewFromScroll();
        });
    },

    updateViewFromScroll() {
        const scrollLeft = this.elements.scrollContainer.scrollLeft;
        const viewStartMs = this.anchorTime.getTime() + scrollLeft * this.msPerPixel;
        this.viewStart = new Date(viewStartMs);
        this.viewEnd = new Date(viewStartMs + this.viewDurationMs);
        this.updateInputs();
        this.render();
        this.scheduleFetchDebounced();
        this.recenterIfNeeded();
        this.notifyViewChange();
    },

    recenterIfNeeded() {
        const threshold = this.viewportWidth * 0.2;
        const maxScroll = this.virtualWidth - this.viewportWidth;
        const scrollLeft = this.elements.scrollContainer.scrollLeft;
        if (scrollLeft < threshold || scrollLeft > Math.max(0, maxScroll - threshold)) {
            this.anchorTime = new Date(
                this.viewStart.getTime() - this.initialScrollLeft * this.msPerPixel,
            );
            this.setScrollLeft(this.initialScrollLeft, true);
            this.updateTimeCursor();
        }
    },

    setScrollLeft(value, skipUpdate = false) {
        this.ignoreScroll = true;
        this.elements.scrollContainer.scrollLeft = value;
        requestAnimationFrame(() => {
            this.ignoreScroll = false;
            if (!skipUpdate) {
                this.updateViewFromScroll();
            }
        });
    },

    getCenterTime() {
        return new Date(this.viewStart.getTime() + this.viewDurationMs / 2);
    },

    get totalHours() {
        return this.viewDurationMs / (1000 * 60 * 60);
    },

    getTimeAtViewportPx(viewportPx) {
        if (!this.elements.scrollContainer) {
            return new Date();
        }
        const clamped = Math.max(0, Math.min(this.viewportWidth, viewportPx));
        const contentPx = this.elements.scrollContainer.scrollLeft + clamped;
        return this.pixelToTime(contentPx);
    },

    onTimeRangeChange() {
        const startVal = this.elements.startInput?.value;
        const endVal = this.elements.endInput?.value;
        if (!startVal || !endVal) {
            return;
        }
        const start = new Date(startVal);
        const end = new Date(endVal);
        if (isNaN(start.getTime()) || isNaN(end.getTime())) {
            this.updateInputs();
            return;
        }
        let duration = end.getTime() - start.getTime();
        if (duration < this.minDurationMs) {
            duration = this.minDurationMs;
        }
        if (duration > this.maxDurationMs) {
            duration = this.maxDurationMs;
        }
        this.viewDurationMs = duration;
        this.msPerPixel = this.viewDurationMs / this.viewportWidth;
        const center = new Date(start.getTime() + this.viewDurationMs / 2);
        this.centerOnTime(center);
    },

    zoomIn() {
        this.adjustZoom(0.5);
    },

    zoomOut() {
        this.adjustZoom(2);
    },

    adjustZoom(multiplier) {
        const proposedDuration = this.viewDurationMs * multiplier;
        const newDuration = Math.min(
            Math.max(proposedDuration, this.minDurationMs),
            this.maxDurationMs,
        );
        if (newDuration === this.viewDurationMs) {
            return;
        }

        const focusViewportPx =
            typeof this.lastPointerViewportX === 'number'
                ? this.lastPointerViewportX
                : this.viewportWidth / 2;
        const focusTime = this.getTimeAtViewportPx(focusViewportPx);

        this.viewDurationMs = newDuration;
        this.msPerPixel = this.viewDurationMs / this.viewportWidth;

        const newStartMs = focusTime.getTime() - focusViewportPx * this.msPerPixel;
        this.viewStart = new Date(newStartMs);
        this.viewEnd = new Date(newStartMs + this.viewDurationMs);
        this.anchorTime = new Date(
            this.viewStart.getTime() - this.elements.scrollContainer.scrollLeft * this.msPerPixel,
        );
        this.updateInputs();
        this.render();
        this.fetchSchedulesForView(true);
        this.notifyViewChange();
        this.updateTimeCursor();
    },

    goToNow() {
        this.centerOnTime(new Date());
    },

    centerOnTime(centerTime) {
        const startMs = centerTime.getTime() - this.viewDurationMs / 2;
        this.viewStart = new Date(startMs);
        this.viewEnd = new Date(startMs + this.viewDurationMs);
        this.anchorTime = new Date(startMs - this.initialScrollLeft * this.msPerPixel);
        this.setScrollLeft(this.initialScrollLeft, true);
        this.updateInputs();
        this.render();
        this.fetchSchedulesForView(true);
        this.notifyViewChange();
    },

    scheduleFetchDebounced() {
        clearTimeout(this.fetchTimeout);
        this.fetchTimeout = setTimeout(() => this.fetchSchedulesForView(false), 200);
    },

    notifyViewChange() {
        if (!this.viewStart || !this.viewEnd) {
            return;
        }
        document.body.dispatchEvent(
            new CustomEvent('timelineViewChanged', {
                detail: {
                    start: this.viewStart.toISOString(),
                    end: this.viewEnd.toISOString(),
                },
            }),
        );
    },

    async fetchSchedulesForView(force) {
        if (!Auth.hasKey()) {
            this.schedules = [];
            this.loadedRange = null;
            this.render();
            return;
        }
        const buffer = this.viewDurationMs;
        const rangeStart = new Date(this.viewStart.getTime() - buffer);
        const rangeEnd = new Date(this.viewEnd.getTime() + buffer);
        if (
            !force &&
            this.loadedRange &&
            rangeStart.getTime() >= this.loadedRange.start &&
            rangeEnd.getTime() <= this.loadedRange.end
        ) {
            return;
        }
        const params = new URLSearchParams();
        params.set('start', rangeStart.toISOString());
        params.set('end', rangeEnd.toISOString());
        const token = ++this.fetchToken;
        try {
            const response = await Auth.fetch(`/api/schedules?${params.toString()}`);
            if (!response.ok) {
                throw new Error('Failed to load timeline data');
            }
            const data = await response.json();
            if (token !== this.fetchToken) {
                return;
            }
            this.schedules = data;
            this.loadedRange = {
                start: rangeStart.getTime(),
                end: rangeEnd.getTime(),
            };
            this.render();
        } catch (err) {
            console.error(err);
        }
    },

    updateInputs() {
        if (this.elements.startInput) {
            this.elements.startInput.value = this.toLocalDatetimeString(this.viewStart);
        }
        if (this.elements.endInput) {
            this.elements.endInput.value = this.toLocalDatetimeString(this.viewEnd);
        }
        if (this.elements.zoomLabel) {
            const hours = this.totalHours;
            if (hours >= 24) {
                const days = Math.round((hours / 24) * 10) / 10;
                this.elements.zoomLabel.textContent = `${days}d`;
            } else {
                this.elements.zoomLabel.textContent = `${Math.round(hours)}h`;
            }
        }
    },

    toLocalDatetimeString(date) {
        const pad = (n) => n.toString().padStart(2, '0');
        return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(
            date.getHours(),
        )}:${pad(date.getMinutes())}`;
    },

    pixelToTime(px) {
        if (!this.anchorTime) {
            return new Date();
        }
        return new Date(this.anchorTime.getTime() + px * this.msPerPixel);
    },

    timeToPx(date) {
        if (!this.anchorTime) {
            return 0;
        }
        const diffMs = date.getTime() - this.anchorTime.getTime();
        return diffMs / this.msPerPixel;
    },

    render() {
        this.renderMarkers();
        this.renderSchedules();
        this.updateNowMarker();
    },

    renderMarkers() {
        const container = this.elements.markers;
        if (!container) return;
        container.innerHTML = '';

        const hours = this.totalHours;
        let intervalHours;
        let majorInterval;
        if (hours <= 2) {
            intervalHours = 0.25;
            majorInterval = 1;
        } else if (hours <= 6) {
            intervalHours = 0.5;
            majorInterval = 2;
        } else if (hours <= 12) {
            intervalHours = 1;
            majorInterval = 6;
        } else if (hours <= 48) {
            intervalHours = 2;
            majorInterval = 12;
        } else if (hours <= 168) {
            intervalHours = 6;
            majorInterval = 24;
        } else {
            intervalHours = 24;
            majorInterval = 24 * 7;
        }

        const intervalMs = intervalHours * 60 * 60 * 1000;
        let currentMs = Math.floor(this.viewStart.getTime() / intervalMs) * intervalMs;
        let current = new Date(currentMs);

        while (current <= this.viewEnd) {
            const isMajor = intervalHours >= 1
                ? current.getHours() % majorInterval === 0 && current.getMinutes() === 0
                : current.getMinutes() === 0;
            const px = this.timeToPx(current);
            if (px >= 0 && px <= this.virtualWidth) {
                const marker = document.createElement('div');
                marker.className = `absolute top-0 bottom-0 border-l ${
                    isMajor ? 'border-zinc-600' : 'border-zinc-800'
                }`;
                marker.style.left = `${px}px`;

                const label = document.createElement('span');
                label.className = `absolute -top-0.5 left-1 text-xs whitespace-nowrap ${
                    isMajor ? 'text-zinc-400' : 'text-zinc-600'
                }`;
                if (isMajor && current.getHours() === 0 && current.getMinutes() === 0) {
                    label.textContent = current.toLocaleDateString(undefined, {
                        month: 'short',
                        day: 'numeric',
                    });
                } else if (intervalHours < 1) {
                    label.textContent = current.toLocaleTimeString(undefined, {
                        hour: '2-digit',
                        minute: '2-digit',
                    });
                } else if (isMajor) {
                    label.textContent = current.toLocaleTimeString(undefined, {
                        hour: '2-digit',
                        minute: '2-digit',
                    });
                } else {
                    label.textContent = current.getHours().toString().padStart(2, '0');
                }
                marker.appendChild(label);
                container.appendChild(marker);
            }
            current = new Date(current.getTime() + intervalMs);
        }
    },

    renderSchedules() {
        const container = this.elements.schedulesContainer;
        if (!container) return;
        container.innerHTML = '';

        for (const schedule of this.schedules) {
            const start = new Date(schedule.start);
            const end = new Date(schedule.end);
            if (end <= this.viewStart || start >= this.viewEnd) {
                continue;
            }
            const startPx = this.timeToPx(start);
            const endPx = this.timeToPx(end);
            const leftPx = Math.max(0, startPx);
            const rightPx = Math.min(this.virtualWidth, endPx);
            const widthPx = Math.max(4, rightPx - leftPx);

            const block = document.createElement('div');
            const isApproved = schedule.status === 'approved';
            block.className = `schedule-block absolute top-8 bottom-4 rounded cursor-pointer transition-all hover:brightness-110 hover:z-20 ${
                isApproved ? 'bg-cyan-500' : 'bg-amber-500'
            }`;
            block.style.left = `${leftPx}px`;
            block.style.width = `${widthPx}px`;
            block.dataset.scheduleId = schedule.id;
            block.title = `${schedule.id}: ${this.formatTime(start)} - ${this.formatTime(end)}`;
            block.addEventListener('click', () => {
                if (typeof window.openDetailModal === 'function') {
                    window.openDetailModal(schedule.id);
                }
            });

            if (widthPx > 60) {
                const label = document.createElement('span');
                label.className = 'absolute inset-0 flex items-center justify-center text-xs font-medium text-white truncate px-1';
                label.textContent = schedule.id;
                block.appendChild(label);
            }

            container.appendChild(block);
        }
    },

    formatTime(date) {
        return date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
    },

    updateHoverLabel(date) {
        const label = this.elements.hoverLabel;
        if (!label) {
            return;
        }
        if (!date) {
            label.classList.add('hidden');
            label.textContent = '';
            return;
        }
        label.classList.remove('hidden');
        label.textContent = this.formatCursorTime(date);
    },

    formatCursorTime(date) {
        return date.toLocaleString(undefined, {
            month: 'short',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit',
        });
    },

    updateNowMarker() {
        const marker = this.elements.nowMarker;
        if (!marker) return;
        const now = new Date();
        if (now < this.viewStart || now > this.viewEnd) {
            marker.classList.add('hidden');
            return;
        }
        marker.classList.remove('hidden');
        marker.style.left = `${this.timeToPx(now)}px`;
    },
};

window.Timeline = Timeline;
document.addEventListener('DOMContentLoaded', () => Timeline.init());

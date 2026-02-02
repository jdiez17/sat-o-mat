// Predictions store - satellite pass predictions
import { formatDateTime, formatTime } from '../utils/datetime.js';

export default {
    items: [],           // Array of Pass objects
    loading: false,
    error: null,
    viewRange: { start: null, end: null },
    loadedRange: null,
    fetchToken: 0,
    fetchTimeout: null,

    // Modal state
    modalOpen: false,
    modalData: null,     // Selected pass details
    selectedTemplate: '',

    setViewRange(start, end) {
        this.viewRange = { start, end };
        this.fetchDebounced();
    },

    fetchDebounced() {
        clearTimeout(this.fetchTimeout);
        this.fetchTimeout = setTimeout(() => this.fetch(), 200);
    },

    async fetch(force = false) {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) {
            this.items = [];
            this.loadedRange = null;
            return;
        }

        if (!this.viewRange.start || !this.viewRange.end) return;

        const viewStart = new Date(this.viewRange.start);
        const viewEnd = new Date(this.viewRange.end);
        const bufferMs = 2 * 24 * 60 * 60 * 1000;
        const viewCenterMs = viewStart.getTime() + (viewEnd.getTime() - viewStart.getTime()) / 2;
        const rangeStart = new Date(viewCenterMs - bufferMs);
        const rangeEnd = new Date(viewCenterMs + bufferMs);

        if (!force && this.loadedRange &&
            viewStart.getTime() >= this.loadedRange.start &&
            viewEnd.getTime() <= this.loadedRange.end) {
            return;
        }

        this.loading = true;
        this.error = null;
        const token = ++this.fetchToken;

        try {
            const params = new URLSearchParams({
                start: rangeStart.toISOString(),
                end: rangeEnd.toISOString(),
            });
            const res = await auth.fetch(`/api/predict?${params}`);
            if (!res.ok) {
                const data = await res.json().catch(() => ({}));
                throw new Error(data.message || data.error || 'Failed to load predictions');
            }
            if (token !== this.fetchToken) return;

            const data = await res.json();
            this.items = data.passes || [];
            this.loadedRange = { start: rangeStart.getTime(), end: rangeEnd.getTime() };
        } catch (e) {
            if (token === this.fetchToken) this.error = e.message;
        } finally {
            if (token === this.fetchToken) this.loading = false;
        }
    },

    openDetail(pass) {
        this.modalData = pass;
        this.modalOpen = true;
    },

    closeModal() {
        this.modalOpen = false;
        this.selectedTemplate = '';
    },

    async createScheduleFromPass() {
        if (!this.modalData || !this.selectedTemplate) return;

        const templateName = this.selectedTemplate;
        const variables = {
            start: this.modalData.aos,
            end: this.modalData.los,
            satellite: this.modalData.satellite,
        };

        this.closeModal();

        Alpine.store('schedules').openEditor({ templateName, variables });
    },

    // Group passes by satellite for timeline rows
    passesBySatellite() {
        const groups = {};
        for (const pass of this.items) {
            if (!groups[pass.satellite]) {
                groups[pass.satellite] = [];
            }
            groups[pass.satellite].push(pass);
        }
        return groups;
    },

    // Get satellites that have passes in current view
    visibleSatellites() {
        if (!this.viewRange.start || !this.viewRange.end) return [];
        const viewStart = new Date(this.viewRange.start);
        const viewEnd = new Date(this.viewRange.end);

        const satellites = new Set();
        for (const pass of this.items) {
            const aos = new Date(pass.aos);
            const los = new Date(pass.los);
            if (!(los <= viewStart || aos >= viewEnd)) {
                satellites.add(pass.satellite);
            }
        }
        return Array.from(satellites).sort();
    },

    // Get passes for a specific satellite in current view
    passesForSatellite(satellite) {
        if (!this.viewRange.start || !this.viewRange.end) return [];
        const viewStart = new Date(this.viewRange.start);
        const viewEnd = new Date(this.viewRange.end);

        return this.items.filter(pass => {
            if (pass.satellite !== satellite) return false;
            const aos = new Date(pass.aos);
            const los = new Date(pass.los);
            return !(los <= viewStart || aos >= viewEnd);
        });
    },

    visiblePasses() {
        if (!this.viewRange.start || !this.viewRange.end) return [];
        const viewStart = new Date(this.viewRange.start);
        const viewEnd = new Date(this.viewRange.end);
        return this.items
            .filter(pass => {
                const aos = new Date(pass.aos);
                const los = new Date(pass.los);
                return !(los <= viewStart || aos >= viewEnd);
            })
            .sort((a, b) => new Date(a.aos) - new Date(b.aos));
    },

    formatTime(value) {
        return formatTime(value);
    },

    formatDateTime(value) {
        return formatDateTime(value);
    },

    formatDuration(seconds) {
        const mins = Math.floor(seconds / 60);
        const secs = seconds % 60;
        return `${mins}m ${secs}s`;
    },

    formatRelativeFromNow(value, now = Date.now()) {
        const date = new Date(value);
        if (isNaN(date.getTime())) return value;
        const diffMs = date.getTime() - now;
        const totalSeconds = Math.floor(Math.abs(diffMs) / 1000);
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = Math.floor((totalSeconds % 3600) / 60);
        const seconds = totalSeconds % 60;
        const pad2 = (value) => value.toString().padStart(2, '0');
        const parts = [];
        if (hours > 0) parts.push(`${hours}h`);
        parts.push(`${pad2(minutes)}m`);
        parts.push(`${pad2(seconds)}s`);
        const label = parts.join(' ');
        return diffMs >= 0 ? label : `${label} ago`;
    },

    nextUpcomingPass() {
        const now = Date.now();
        return this.items
            .filter(pass => new Date(pass.aos).getTime() > now)
            .sort((a, b) => new Date(a.aos) - new Date(b.aos))[0] || null;
    },
};

// Schedules store - centralized schedule data management
import { formatDateTime, formatTime } from '../utils/datetime.js';

export default {
    items: [],
    loading: false,
    error: null,
    viewRange: { start: null, end: null },
    loadedRange: null,
    fetchToken: 0,
    fetchTimeout: null,

    // Unified modal state
    modalOpen: false,
    modalMode: 'new', // 'view', 'edit', 'new'
    modalLoading: false,
    modalError: null,
    modalData: null, // { schedule, content, variables } for view mode

    // Templates
    templates: [],
    templatesLoading: false,
    templatesError: null,

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
            const res = await auth.fetch(`/api/schedules?${params}`);
            if (!res.ok) {
                const data = await res.json().catch(() => ({}));
                throw new Error(data.message || data.error || 'Failed to load');
            }
            if (token !== this.fetchToken) return;

            this.items = await res.json();
            this.loadedRange = { start: rangeStart.getTime(), end: rangeEnd.getTime() };
        } catch (e) {
            if (token === this.fetchToken) this.error = e.message;
        } finally {
            if (token === this.fetchToken) this.loading = false;
        }
    },

    async approve(id) {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) return;
        try {
            const res = await auth.fetch(`/api/schedules/${id}/approve`, { method: 'POST' });
            if (!res.ok) throw new Error('Failed to approve');
            this.loadedRange = null;
            await this.fetch(true);
            if (this.modalOpen && this.modalMode === 'view' && this.modalData?.schedule.id === id) {
                await this.openDetail(id);
            }
        } catch (e) {
            alert(e.message);
        }
    },

    async reject(id) {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) return;
        try {
            const res = await auth.fetch(`/api/schedules/${id}/reject`, { method: 'POST' });
            if (!res.ok) throw new Error('Failed to reject');
            this.loadedRange = null;
            await this.fetch(true);
            if (this.modalOpen && this.modalMode === 'view' && this.modalData?.schedule.id === id) {
                this.closeModal();
            }
        } catch (e) {
            alert(e.message);
        }
    },

    async remove(id) {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) return;
        try {
            const res = await auth.fetch(`/api/schedules/${id}`, { method: 'DELETE' });
            if (!res.ok) throw new Error('Failed to delete');
            this.loadedRange = null;
            await this.fetch(true);
            if (this.modalOpen && this.modalMode === 'view' && this.modalData?.schedule.id === id) {
                this.closeModal();
            }
        } catch (e) {
            alert(e.message);
        }
    },

    // Open modal in view mode
    async openDetail(id) {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) {
            auth.showModal();
            return;
        }

        this.modalData = null;
        this.modalError = null;
        this.modalLoading = true;
        this.modalMode = 'view';
        this.modalOpen = true;

        try {
            const res = await auth.fetch(`/api/schedules/${id}`);
            if (!res.ok) throw new Error('Failed to load');
            const data = await res.json();
            const { content, variables, ...schedule } = data;
            this.modalData = {
                schedule: schedule,
                content: content || '',
                variables: variables || []
            };
        } catch (e) {
            this.modalError = e.message;
        } finally {
            this.modalLoading = false;
        }
    },

    // Open modal in new/edit mode
    openEditor(opts = {}) {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) {
            auth.showModal();
            return;
        }

        this.modalError = null;
        this.modalLoading = false;
        this.modalMode = 'new';

        if (opts.templateName) {
            this.modalData = {
                templateName: opts.templateName,
                variables: opts.variables || {},
            };
        } else {
            this.modalData = null;
        }

        this.modalOpen = true;
    },

    closeModal() {
        this.modalOpen = false;
    },

    formatDate(value, options = {}) {
        return options.omitDate ? formatTime(value) : formatDateTime(value);
    },

    formatDuration(start, end) {
        const diffMs = Math.max(0, new Date(end).getTime() - new Date(start).getTime());
        const totalMinutes = Math.round(diffMs / 60000);
        const hours = Math.floor(totalMinutes / 60);
        const minutes = totalMinutes % 60;
        return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
    },

    visibleItems() {
        if (!this.viewRange.start || !this.viewRange.end) return [];
        const viewStart = new Date(this.viewRange.start);
        const viewEnd = new Date(this.viewRange.end);
        return this.items
            .filter(schedule => {
                const start = new Date(schedule.start);
                const end = new Date(schedule.end);
                return !(end <= viewStart || start >= viewEnd);
            })
            .sort((a, b) => new Date(a.start) - new Date(b.start));
    },

    async fetchTemplates() {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) {
            this.templates = [];
            return;
        }

        this.templatesLoading = true;
        this.templatesError = null;

        try {
            const res = await auth.fetch('/api/schedules/templates');
            if (!res.ok) {
                const data = await res.json().catch(() => ({}));
                throw new Error(data.message || data.error || 'Failed to load templates');
            }
            this.templates = await res.json();
        } catch (e) {
            this.templatesError = e.message;
            this.templates = [];
        } finally {
            this.templatesLoading = false;
        }
    },

    async fetchTemplate(name) {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) {
            throw new Error('Not authenticated');
        }

        try {
            const res = await auth.fetch(`/api/schedules/template/${encodeURIComponent(name)}`);
            if (!res.ok) {
                const data = await res.json().catch(() => ({}));
                throw new Error(data.message || data.error || 'Failed to load template');
            }
            return await res.text();
        } catch (e) {
            throw new Error(`Failed to load template: ${e.message}`);
        }
    },
};

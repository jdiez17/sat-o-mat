// Schedules store - centralized schedule data management
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
        const buffer = viewEnd.getTime() - viewStart.getTime();
        const rangeStart = new Date(viewStart.getTime() - buffer);
        const rangeEnd = new Date(viewEnd.getTime() + buffer);

        if (!force && this.loadedRange &&
            rangeStart.getTime() >= this.loadedRange.start &&
            rangeEnd.getTime() <= this.loadedRange.end) {
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

        // Clear previous state first
        this.modalData = null;
        this.modalError = null;
        this.modalLoading = true;
        this.modalMode = 'view';
        this.modalOpen = true;

        try {
            const res = await auth.fetch(`/api/schedules/${id}`);
            if (!res.ok) throw new Error('Failed to load');
            const data = await res.json();
            this.modalData = {
                schedule: data.schedule,
                content: data.content || '',
                variables: data.variables || []
            };
        } catch (e) {
            this.modalError = e.message;
        } finally {
            this.modalLoading = false;
        }
    },

    // Open modal in new/edit mode
    openEditor() {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) {
            auth.showModal();
            return;
        }

        // Clear previous state
        this.modalData = null;
        this.modalError = null;
        this.modalLoading = false;
        this.modalMode = 'new';
        this.modalOpen = true;
    },

    closeModal() {
        this.modalOpen = false;
        // Don't clear data immediately - wait for transition to finish
        // Data will be cleared when opening next time
    },

    formatDate(value, options = {}) {
        const date = new Date(value);
        if (isNaN(date.getTime())) return value;
        const opts = options.omitDate
            ? { hour: '2-digit', minute: '2-digit' }
            : { year: 'numeric', month: 'short', day: '2-digit', hour: '2-digit', minute: '2-digit' };
        return date.toLocaleString(undefined, opts);
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
};

import { formatDateTime } from '../utils/datetime.js';

const DEFAULT_TLE = `ISS (ZARYA)
1 25544U 98067A   26012.17690827  .00009276  00000-0  17471-3 0  9998
2 25544  51.6333 351.7881 0007723   8.9804 351.1321 15.49250518547578`;

export default {
    mode: null,
    sample: null,
    trajectory: [],
    loading: false,
    error: null,
    pollTimer: null,
    pollTick: 0,

    modalOpen: false,
    modalTle: DEFAULT_TLE,
    modalEndLocal: '',

    init() {
        this.fetchMode().then(() => {
            if (this.isRunning()) {
                this.fetchTrajectory();
                this.startPolling();
            }
        });
    },

    isRunning() {
        return !!this.mode && !!this.mode.Running;
    },

    showModal() {
        this.modalTle = DEFAULT_TLE;
        this.modalEndLocal = '';
        this.modalOpen = true;
    },

    hideModal() {
        this.modalOpen = false;
    },

    formatDate(value) {
        if (!value) return '---';
        const formatted = formatDateTime(value);
        return formatted === value ? '---' : formatted;
    },

    formatFrequency(value) {
        if (value === null || value === undefined) return '---';
        if (value >= 1e9) return `${(value / 1e9).toFixed(3)} GHz`;
        if (value >= 1e6) return `${(value / 1e6).toFixed(3)} MHz`;
        if (value >= 1e3) return `${(value / 1e3).toFixed(3)} kHz`;
        return `${value.toFixed(0)} Hz`;
    },

    trajectoryPoints(size = 128, margin = 8) {
        if (!this.trajectory.length) return '';
        const radius = size / 2 - margin;
        const center = size / 2;
        return this.trajectory
            .map((point) => this.polarPoint(point.azimuth_deg, point.elevation_deg, center, radius))
            .join(' ');
    },

    currentPoint(size = 128, margin = 8) {
        if (!this.sample) return null;
        if (this.sample.elevation_deg < 0) return null;
        const radius = size / 2 - margin;
        const center = size / 2;
        return this.polarPoint(this.sample.azimuth_deg, this.sample.elevation_deg, center, radius);
    },

    polarPoint(azDeg, elDeg, center, radius) {
        const azRad = (azDeg - 90) * (Math.PI / 180);
        const r = radius * (1 - Math.min(Math.max(elDeg, 0), 90) / 90);
        const x = center + r * Math.cos(azRad);
        const y = center + r * Math.sin(azRad);
        return `${x.toFixed(2)},${y.toFixed(2)}`;
    },

    async start() {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) {
            auth.showModal();
            return;
        }
        this.loading = true;
        this.error = null;
        try {
            const end = this.modalEndLocal ? new Date(this.modalEndLocal).toISOString() : null;
            const res = await auth.fetch('/api/tracker/run', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    tle: this.modalTle.trim(),
                    end,
                    radio: null,
                }),
            });
            if (!res.ok) {
                const data = await res.json().catch(() => ({}));
                throw new Error(data.message || data.error || 'Failed to start');
            }
            this.mode = await res.json();
            this.hideModal();
            await this.fetchTrajectory();
            this.startPolling();
        } catch (e) {
            this.error = e.message;
        } finally {
            this.loading = false;
        }
    },

    async stop() {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) return;
        this.loading = true;
        this.error = null;
        try {
            const res = await auth.fetch('/api/tracker/stop', { method: 'POST' });
            if (!res.ok) {
                const data = await res.json().catch(() => ({}));
                throw new Error(data.message || data.error || 'Failed to stop');
            }
            this.mode = await res.json();
            this.sample = null;
            this.trajectory = [];
            this.stopPolling();
        } catch (e) {
            this.error = e.message;
        } finally {
            this.loading = false;
        }
    },

    async fetchMode() {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) return;
        const res = await auth.fetch('/api/tracker/status/mode');
        if (!res.ok) return;
        this.mode = await res.json();
        if (this.isRunning() && !this.pollTimer) {
            await this.fetchTrajectory();
            this.startPolling();
        }
    },

    async fetchSample() {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) return;
        const res = await auth.fetch('/api/tracker/status/sample');
        if (!res.ok) return;
        this.sample = await res.json();
    },

    async fetchTrajectory() {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) return;
        const res = await auth.fetch('/api/tracker/status/trajectory');
        if (!res.ok) return;
        this.trajectory = await res.json();
    },

    startPolling() {
        this.stopPolling();
        this.pollTick = 0;
        this.pollTimer = setInterval(async () => {
            this.pollTick += 1;
            await this.fetchSample();
            if (this.pollTick % 5 === 0) await this.fetchMode();
            if (this.pollTick % 10 === 0) await this.fetchTrajectory();
        }, 1000);
    },

    stopPolling() {
        if (this.pollTimer) {
            clearInterval(this.pollTimer);
            this.pollTimer = null;
        }
    },
};

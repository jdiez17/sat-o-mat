// Dashboard script handles schedule list rendering and modal interactions

const Dashboard = {
    elements: {
        scheduleListRoot: null,
        scheduleModal: null,
        detailModal: null,
        detailContent: null,
    },
    schedules: [],
    viewRange: null,
    listLoading: false,
    listError: null,

    init() {
        this.elements.scheduleListRoot = document.getElementById('schedule-list-root');
        this.elements.scheduleModal = document.getElementById('schedule-modal');
        this.elements.detailModal = document.getElementById('detail-modal');
        this.elements.detailContent = document.getElementById('detail-content');

        if (!this.elements.scheduleListRoot) {
            return;
        }

        this.bindEvents();
        this.renderScheduleList();
    },

    bindEvents() {
        document.body.addEventListener('scheduleUpdated', () => this.refreshSchedules(true));
        document.body.addEventListener('authChanged', () => this.refreshSchedules(true));
        document.body.addEventListener('timelineViewChanged', (event) => {
            this.viewRange = event.detail;
            this.refreshSchedules(true);
        });
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                this.closeScheduleModal();
                this.closeDetailModal();
            }
        });
    },

    async refreshSchedules(force) {
        if (!Auth.hasKey()) {
            this.schedules = [];
            this.listError = null;
            this.renderScheduleList();
            return;
        }
        if (!this.viewRange) {
            this.schedules = [];
            this.listError = null;
            this.renderScheduleList();
            return;
        }
        if (!force && this.listLoading) {
            return;
        }
        this.listLoading = true;
        this.listError = null;
        this.renderScheduleList();
        try {
            const params = new URLSearchParams();
            params.set('start', this.viewRange.start);
            params.set('end', this.viewRange.end);
            const url = `/api/schedules?${params.toString()}`;
            const response = await Auth.fetch(url);
            if (!response.ok) {
                const error = await response.json().catch(() => ({}));
                throw new Error(error.message || error.error || 'Failed to load schedules');
            }
            this.schedules = await response.json();
        } catch (err) {
            this.listError = err.message;
        } finally {
            this.listLoading = false;
            this.renderScheduleList();
        }
    },

    renderScheduleList() {
        const container = this.elements.scheduleListRoot;
        if (!container) {
            return;
        }
        if (!Auth.hasKey()) {
            container.innerHTML =
                '<div class="text-center py-4 text-zinc-500">Connect your API key to load schedules.</div>';
            return;
        }
        if (!this.viewRange) {
            container.innerHTML =
                '<div class="text-center py-4 text-zinc-500">Adjust the timeline to choose a time range.</div>';
            return;
        }
        if (this.listLoading) {
            container.innerHTML = '<div class="text-center py-4 text-zinc-500">Loading schedules...</div>';
            return;
        }
        if (this.listError) {
            container.innerHTML = `<div class="text-center py-4 text-red-400">${this.listError}</div>`;
            return;
        }
        if (!this.schedules.length) {
            container.innerHTML = '<div class="text-center py-8 text-zinc-500">No schedules found.</div>';
            return;
        }

        const rows = this.schedules
            .map((schedule) => this.renderScheduleRow(schedule))
            .join('');

        container.innerHTML = `
            <table class="w-full text-sm">
                <thead>
                    <tr class="text-left text-zinc-400 border-b border-zinc-800">
                        <th class="px-3 py-2 font-medium">Status</th>
                        <th class="px-3 py-2 font-medium">ID</th>
                        <th class="px-3 py-2 font-medium">Time</th>
                        <th class="px-3 py-2 font-medium text-right">Actions</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-zinc-800">${rows}</tbody>
            </table>
        `;

        container.querySelectorAll('[data-action]').forEach((button) => {
            button.addEventListener('click', (event) => this.onScheduleAction(event));
        });
    },

    renderScheduleRow(schedule) {
        const statusPill =
            schedule.status === 'approved'
                ? '<span class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium bg-cyan-500/20 text-cyan-400"><span class="w-1.5 h-1.5 rounded-full bg-cyan-400"></span>Active</span>'
                : '<span class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium bg-amber-500/20 text-amber-400"><span class="w-1.5 h-1.5 rounded-full bg-amber-400"></span>Pending</span>';
        const start = this.formatDate(schedule.start);
        const end = this.formatDate(schedule.end, { omitDate: true });
        return `
            <tr class="hover:bg-zinc-800/50 transition-colors">
                <td class="px-3 py-3">${statusPill}</td>
                <td class="px-3 py-3 font-mono text-zinc-300 text-xs">${schedule.id}</td>
                <td class="px-3 py-3 text-zinc-400">${start} - ${end}</td>
                <td class="px-3 py-3">
                    <div class="flex items-center justify-end gap-2">
                        <button class="p-1.5 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-700 rounded transition-colors" data-action="view" data-id="${schedule.id}" title="View details">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/></svg>
                        </button>
                        ${
                            schedule.status === 'pending'
                                ? `<button class="p-1.5 text-green-400 hover:text-green-300 hover:bg-green-500/20 rounded transition-colors" data-action="approve" data-id="${schedule.id}" title="Approve">
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
                                   </button>
                                   <button class="p-1.5 text-red-400 hover:text-red-300 hover:bg-red-500/20 rounded transition-colors" data-action="reject" data-id="${schedule.id}" title="Reject">
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                                   </button>`
                                : ''
                        }
                        <button class="p-1.5 text-zinc-400 hover:text-red-400 hover:bg-red-500/20 rounded transition-colors" data-action="delete" data-id="${schedule.id}" title="Delete">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                        </button>
                    </div>
                </td>
            </tr>
        `;
    },

    onScheduleAction(event) {
        const button = event.currentTarget;
        const action = button.dataset.action;
        const id = button.dataset.id;
        if (!action || !id) {
            return;
        }
        if (action === 'view') {
            this.openDetailModal(id);
            return;
        }
        if (!Auth.hasKey()) {
            Auth.showModal();
            return;
        }
        if (action === 'delete') {
            const confirmed = window.confirm('Delete this schedule?');
            if (!confirmed) {
                return;
            }
            this.performScheduleMutation(`/api/schedules/${id}`, { method: 'DELETE' });
        } else if (action === 'approve') {
            this.performScheduleMutation(`/api/schedules/${id}/approve`, { method: 'POST' });
        } else if (action === 'reject') {
            this.performScheduleMutation(`/api/schedules/${id}/reject`, { method: 'POST' });
        }
    },

    async performScheduleMutation(url, options) {
        try {
            const response = await Auth.fetch(url, options);
            if (!response.ok) {
                const error = await response.json().catch(() => ({}));
                throw new Error(error.message || error.error || 'Request failed');
            }
            document.body.dispatchEvent(new CustomEvent('scheduleUpdated'));
            if (this.elements.detailModal && !this.elements.detailModal.classList.contains('hidden')) {
                const idMatch = /\/api\/schedules\/(.*?)(\/|$)/.exec(url);
                const scheduleId = idMatch ? idMatch[1] : null;
                if (scheduleId && options.method !== 'DELETE') {
                    this.openDetailModal(scheduleId);
                } else if (options.method === 'DELETE') {
                    this.closeDetailModal();
                }
            }
        } catch (err) {
            alert(err.message || 'Operation failed');
        }
    },

    openScheduleModal() {
        if (!Auth.hasKey()) {
            Auth.showModal();
            return;
        }
        this.elements.scheduleModal?.classList.remove('hidden');
    },

    closeScheduleModal() {
        this.elements.scheduleModal?.classList.add('hidden');
    },

    async openDetailModal(scheduleId) {
        if (!Auth.hasKey()) {
            Auth.showModal();
            return;
        }
        this.elements.detailModal?.classList.remove('hidden');
        if (this.elements.detailContent) {
            this.elements.detailContent.innerHTML =
                '<div class="p-6 text-center text-zinc-500">Loading details...</div>';
        }
        try {
            const response = await Auth.fetch(`/api/schedules/${scheduleId}`);
            if (!response.ok) {
                const error = await response.json().catch(() => ({}));
                throw new Error(error.message || error.error || 'Failed to load details');
            }
            const data = await response.json();
            this.renderDetailContent(data.schedule, data.content, data.variables || []);
        } catch (err) {
            if (this.elements.detailContent) {
                this.elements.detailContent.innerHTML = `<div class="p-6 text-center text-red-400">${
                    err.message || 'Failed to load details'
                }</div>`;
            }
        }
    },

    renderDetailContent(schedule, content, variables) {
        if (!this.elements.detailContent) {
            return;
        }
        const duration = this.describeDuration(schedule.start, schedule.end);
        const statusLabel =
            schedule.status === 'approved'
                ? '<span class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium bg-cyan-500/20 text-cyan-400">Active</span>'
                : '<span class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium bg-amber-500/20 text-amber-400">Pending</span>';

        this.elements.detailContent.innerHTML = `
            <div class="flex items-center justify-between px-4 py-3 border-b border-zinc-800">
                <div class="flex items-center gap-3">
                    <h2 class="text-lg font-medium">Schedule Details</h2>
                    ${statusLabel}
                </div>
                <button class="p-2 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 rounded transition-colors" data-detail-action="close">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                </button>
            </div>
            <div class="flex-1 overflow-auto p-4 space-y-6">
                <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <div>
                        <div class="text-xs text-zinc-500 uppercase tracking-wide mb-1">ID</div>
                        <div class="font-mono text-sm text-zinc-300 break-all">${schedule.id}</div>
                    </div>
                    <div>
                        <div class="text-xs text-zinc-500 uppercase tracking-wide mb-1">Start</div>
                        <div class="text-sm text-zinc-300">${this.formatDate(schedule.start)}</div>
                    </div>
                    <div>
                        <div class="text-xs text-zinc-500 uppercase tracking-wide mb-1">End</div>
                        <div class="text-sm text-zinc-300">${this.formatDate(schedule.end)}</div>
                    </div>
                    <div>
                        <div class="text-xs text-zinc-500 uppercase tracking-wide mb-1">Duration</div>
                        <div class="text-sm text-zinc-300">${duration}</div>
                    </div>
                </div>
                <div>
                    <h3 class="text-sm font-medium text-zinc-400 mb-2">Schedule YAML</h3>
                    <div class="bg-zinc-950 rounded border border-zinc-800 overflow-hidden">
                        <div id="detail-yaml-view" class="h-64"></div>
                    </div>
                </div>
                <div class="border border-zinc-800 rounded-lg p-3 bg-zinc-950">
                    <div class="flex items-center justify-between mb-2">
                        <div>
                            <h3 class="text-sm font-medium text-zinc-100">Executed Variables</h3>
                            <p class="text-xs text-zinc-500">Resolved values that were used when this schedule ran.</p>
                        </div>
                        <span class="text-xs text-zinc-500">${variables.length} defined</span>
                    </div>
                    <div class="grid gap-2 md:grid-cols-2" id="detail-variable-list">
                        ${
                            variables.length
                                ? variables.map((variable) => this.renderVariableEntry(variable)).join('')
                                : '<div class="text-sm text-zinc-500 col-span-full">No custom variables.</div>'
                        }
                    </div>
                </div>
                <div class="text-sm text-zinc-500">
                    Execution logs and artifacts will appear here once the executor publishes them.
                </div>
            </div>
            <div class="flex items-center justify-between px-4 py-3 border-t border-zinc-800">
                <button class="px-3 py-1.5 text-sm text-red-400 hover:text-red-300 hover:bg-red-500/10 rounded transition-colors" data-detail-action="delete" data-id="${schedule.id}">
                    Delete Schedule
                </button>
                ${
                    schedule.status === 'pending'
                        ? `<div class="flex items-center gap-2">
                                <button class="px-4 py-1.5 text-sm text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 rounded transition-colors" data-detail-action="reject" data-id="${schedule.id}">
                                    Reject
                                </button>
                                <button class="px-4 py-1.5 text-sm bg-cyan-600 hover:bg-cyan-500 text-white rounded transition-colors" data-detail-action="approve" data-id="${schedule.id}">
                                    Approve
                                </button>
                           </div>`
                        : '<div></div>'
                }
            </div>
        `;

        this.attachDetailActions();
        this.mountYamlViewer(content);
    },

    renderVariableEntry(variable) {
        const value = variable.value || '';
        const isMultiline = value.includes('\n');
        const content = isMultiline
            ? `<pre class="mt-1 font-mono text-xs text-zinc-200 whitespace-pre-wrap break-words bg-zinc-900 rounded p-2 border border-zinc-800">${value}</pre>`
            : `<div class="mt-1 font-mono text-sm text-zinc-200 break-all">${value}</div>`;
        return `
            <div class="border border-zinc-800 rounded-lg p-3 bg-zinc-950">
                <div class="text-xs uppercase tracking-wide text-zinc-500">${variable.name}</div>
                ${content}
            </div>
        `;
    },

    attachDetailActions() {
        this.elements.detailContent?.querySelectorAll('[data-detail-action]').forEach((button) => {
            button.addEventListener('click', (event) => {
                const action = button.dataset.detailAction;
                const id = button.dataset.id;
                if (action === 'close') {
                    this.closeDetailModal();
                } else if (action === 'delete' && id) {
                    const confirmed = window.confirm('Delete this schedule?');
                    if (confirmed) {
                        this.performScheduleMutation(`/api/schedules/${id}`, { method: 'DELETE' });
                    }
                } else if (action === 'approve' && id) {
                    this.performScheduleMutation(`/api/schedules/${id}/approve`, { method: 'POST' });
                } else if (action === 'reject' && id) {
                    this.performScheduleMutation(`/api/schedules/${id}/reject`, { method: 'POST' });
                }
            });
        });
    },

    mountYamlViewer(content) {
        const container = document.getElementById('detail-yaml-view');
        if (!container) {
            return;
        }
        if (window.CodeMirror) {
            new CodeMirror.EditorView({
                doc: content,
                extensions: [
                    CodeMirror.basicSetup,
                    CodeMirror.yaml(),
                    CodeMirror.oneDark,
                    CodeMirror.EditorView.editable.of(false),
                ],
                parent: container,
            });
        } else {
            container.textContent = content;
        }
    },

    closeDetailModal() {
        this.elements.detailModal?.classList.add('hidden');
    },

    formatDate(value, options = {}) {
        const date = new Date(value);
        if (isNaN(date.getTime())) {
            return value;
        }
        const opts = options.omitDate
            ? { hour: '2-digit', minute: '2-digit' }
            : {
                  year: 'numeric',
                  month: 'short',
                  day: '2-digit',
                  hour: '2-digit',
                  minute: '2-digit',
              };
        return date.toLocaleString(undefined, opts);
    },

    describeDuration(start, end) {
        const startDate = new Date(start);
        const endDate = new Date(end);
        const diffMs = Math.max(0, endDate.getTime() - startDate.getTime());
        const totalMinutes = Math.round(diffMs / (1000 * 60));
        const hours = Math.floor(totalMinutes / 60);
        const minutes = totalMinutes % 60;
        if (hours > 0) {
            return `${hours}h ${minutes}m`;
        }
        return `${minutes}m`;
    },
};

window.openNewScheduleModal = () => Dashboard.openScheduleModal();
window.closeNewScheduleModal = () => Dashboard.closeScheduleModal();
window.openDetailModal = (id) => Dashboard.openDetailModal(id);
window.closeDetailModal = () => Dashboard.closeDetailModal();

document.addEventListener('DOMContentLoaded', () => Dashboard.init());

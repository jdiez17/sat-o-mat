import { formatDateTime } from '../utils/datetime.js';

// Unified Schedule Modal component (view/edit/new)
export default () => ({
    yamlEditor: null,
    yamlExpanded: true,
    variablesExpanded: true,
    variables: {},
    variablesStatus: 'Awaiting file...',
    validationState: 'idle',
    validationMessage: '',
    submitting: false,
    validationTimer: null,
    selectedTemplate: '',

    get store() {
        return Alpine.store('schedules');
    },

    get isViewMode() {
        return this.store.modalMode === 'view';
    },

    get validationColor() {
        const colors = { success: 'text-green-400', error: 'text-rose-400', pending: 'text-amber-400' };
        return colors[this.validationState] || 'text-zinc-400';
    },

    get validationPrefix() {
        const prefixes = { success: 'Valid:', error: 'Error:', pending: 'Validating:' };
        return prefixes[this.validationState] || '';
    },

    get canSubmit() {
        return this.validationState === 'success' && !this.submitting;
    },

    get sortedVariables() {
        const entries = Object.entries(this.variables);
        const order = ['start', 'end'];
        return [
            ...order.filter((k) => k in this.variables).map((key) => [key, this.variables[key]]),
            ...entries.filter(([key]) => !order.includes(key)).sort(([a], [b]) => a.localeCompare(b)),
        ];
    },

    init() {
        this.$watch('yamlExpanded', (expanded) => {
            if (expanded && !this.yamlEditor && this.store.modalOpen) {
                this.$nextTick(() => this.initYamlEditor());
            }
        });

        this.$watch('$store.schedules.modalData', (data) => {
            if (data && this.isViewMode && this.yamlExpanded && !this.yamlEditor) {
                this.$nextTick(() => this.initYamlEditor());
            }
        });

        this.$watch('$store.schedules.modalOpen', (isOpen) => {
            if (isOpen) {
                this.$nextTick(() => this.setupModal());
            } else {
                this.cleanup();
            }
        });

        if (this.store.modalOpen) {
            this.$nextTick(() => this.setupModal());
        }
    },

    setupModal() {
        this.yamlExpanded = true;
        this.variablesExpanded = true;
        this.selectedTemplate = '';

        if (this.isViewMode) {
            if (this.store.modalData?.content) {
                this.$nextTick(() => this.initYamlEditor());
            }
        } else {
            if (this.store.modalMode === 'new') {
                this.store.fetchTemplates();
            }
            this.$nextTick(() => {
                this.initYamlEditor();
                if (this.store.modalData?.templateName) {
                    this.selectedTemplate = this.store.modalData.templateName;
                    this.loadTemplate(this.store.modalData.variables || {});
                }
            });
        }
    },

    initYamlEditor() {
        if (!this.store.modalOpen || this.store.modalLoading || !window.CodeMirror) return;

        const container = this.$refs.yamlEditor;
        if (!container) return;

        if (this.yamlEditor) {
            this.yamlEditor.destroy();
            this.yamlEditor = null;
        }
        container.innerHTML = '';

        const content = this.isViewMode
            ? this.extractSteps(this.store.modalData.content)
            : '';

        const extensions = [
            window.CodeMirror.basicSetup,
            window.CodeMirror.yaml(),
            window.CodeMirror.oneDark,
        ];

        if (this.isViewMode) {
            extensions.push(
                window.CodeMirror.EditorView.editable.of(false),
                window.CodeMirror.EditorView.theme({
                    "&": { fontSize: "13px" },
                    ".cm-scroller": { overflow: "auto" },
                    ".cm-content": { padding: "8px 0" }
                })
            );
        } else {
            extensions.push(
                window.CodeMirror.EditorView.updateListener.of((update) => {
                    if (update.docChanged) this.scheduleValidation();
                })
            );
        }

        this.yamlEditor = new window.CodeMirror.EditorView({
            doc: content || '',
            extensions,
            parent: container,
        });
    },

    extractSteps(fullYaml) {
        try {
            const doc = window.jsyaml.load(fullYaml);
            return doc?.steps ? window.jsyaml.dump(doc.steps, { lineWidth: 100 }) : '[]';
        } catch {
            return fullYaml;
        }
    },

    cleanup() {
        if (this.yamlEditor) {
            this.yamlEditor.destroy();
            this.yamlEditor = null;
        }
        this.variables = {};
        this.variablesStatus = 'Awaiting file...';
        this.validationState = 'idle';
        this.validationMessage = '';
        this.submitting = false;
        this.yamlExpanded = true;
        this.variablesExpanded = true;
        this.selectedTemplate = '';
    },

    showVariables() {
        return this.isViewMode
            ? this.store.modalData?.variables?.length > 0
            : Object.keys(this.variables).length > 0;
    },

    getVariableCount() {
        return this.isViewMode
            ? this.store.modalData?.variables?.length || 0
            : Object.keys(this.variables).length;
    },

    async loadTemplate(overrideVars = {}) {
        if (!this.selectedTemplate) {
            this.showValidation('error', 'Please select a template first');
            return;
        }

        try {
            const content = await this.store.fetchTemplate(this.selectedTemplate);
            const doc = window.jsyaml.load(content);
            const mergedDoc = {
                ...doc,
                variables: { ...(doc.variables || {}), ...overrideVars }
            };
            this.populateFromDocument(mergedDoc);
            this.variablesStatus = this.selectedTemplate;
        } catch (e) {
            this.showValidation('error', e.message);
        }
    },

    async handleFileUpload(event) {
        const file = event.target.files?.[0];
        if (!file) return;
        try {
            const doc = window.jsyaml.load(await file.text());
            this.populateFromDocument(doc);
            this.variablesStatus = file.name;
        } catch {
            this.showValidation('error', 'Failed to parse file.');
        }
    },

    populateFromDocument(doc) {
        if (!doc || typeof doc !== 'object') return;

        const variables = { ...(doc.variables || {}) };
        if (!variables.start) variables.start = new Date(Date.now() + 3600000).toISOString();
        if (!variables.end) variables.end = new Date(new Date(variables.start).getTime() + 900000).toISOString();
        this.variables = variables;

        if (this.yamlEditor) {
            const yaml = window.jsyaml.dump(doc.steps || [], { lineWidth: 100 });
            this.yamlEditor.dispatch({
                changes: { from: 0, to: this.yamlEditor.state.doc.length, insert: yaml.trim() || '- {}' },
            });
        }
        this.scheduleValidation();
    },

    isDatetimeVar(name) {
        return name === 'start' || name === 'end';
    },

    isMultilineVar(name) {
        return typeof this.variables[name] === 'string' && this.variables[name].includes('\n');
    },

    getVariableInputValue(name) {
        const value = this.variables[name];
        return this.isDatetimeVar(name) ? this.isoToUtcInput(value) : value ?? '';
    },

    onVariableInput(name, event) {
        this.variables[name] = this.isDatetimeVar(name)
            ? this.utcInputToIso(event.target.value) || ''
            : event.target.value;
        this.scheduleValidation();
    },

    applyPreset(name, offsetMinutes) {
        const d = new Date();
        d.setUTCMinutes(d.getUTCMinutes() + offsetMinutes);
        this.variables[name] = d.toISOString();

        this.scheduleValidation();
    },

    scheduleValidation() {
        clearTimeout(this.validationTimer);
        this.validationTimer = setTimeout(() => this.validate(), 400);
    },

    buildScheduleYaml() {
        const stepsText = this.yamlEditor?.state.doc.toString() || '';
        let stepsAst = [];
        if (stepsText.trim()) {
            const parsed = window.jsyaml.load(stepsText);
            if (!Array.isArray(parsed)) throw new Error('Steps must be a YAML list.');
            stepsAst = parsed;
        }
        return window.jsyaml.dump({ variables: { ...this.variables }, steps: stepsAst }, { lineWidth: 100 });
    },

    async validate() {
        let yaml;
        try {
            yaml = this.buildScheduleYaml();
        } catch (e) {
            this.showValidation('error', e.message);
            return;
        }

        const auth = Alpine.store('auth');
        if (!auth.hasKey()) {
            this.showValidation('info', 'Enter an API key to validate.');
            return;
        }

        this.showValidation('pending', 'Validating...');

        try {
            const res = await auth.fetch('/api/schedules/validate', {
                method: 'POST',
                headers: { 'Content-Type': 'application/yaml' },
                body: yaml,
            });
            const data = await res.json();
            if (!res.ok) throw new Error(data.message || data.error || 'Validation failed');

            if (data.valid) {
                const range = data.start && data.end
                    ? `${this.formatDisplayDate(data.start)} → ${this.formatDisplayDate(data.end)}`
                    : 'Ready to submit.';
                this.showValidation('success', range);
            } else {
                this.showValidation('error', data.errors?.[0] || 'Validation failed.');
            }
        } catch (e) {
            this.showValidation('error', e.message);
        }
    },

    showValidation(state, message) {
        this.validationState = state;
        this.validationMessage = message;
    },

    getScheduleName() {
        // variablesStatus holds the template or file name (e.g. "nanoff_sband_rx.yml")
        const source = this.variablesStatus || '';
        // Strip file extension (.yml, .yaml) to get the basename
        return source.replace(/\.(ya?ml)$/i, '');
    },

    async submit() {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) {
            auth.showModal();
            return;
        }

        let yaml;
        try {
            yaml = this.buildScheduleYaml();
        } catch (e) {
            this.showValidation('error', e.message);
            return;
        }

        this.submitting = true;
        try {
            const name = this.getScheduleName();
            const url = name
                ? `/api/schedules?name=${encodeURIComponent(name)}`
                : '/api/schedules';
            const res = await auth.fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/yaml' },
                body: yaml,
            });
            if (!res.ok) throw new Error('Submit failed');

            this.store.loadedRange = null;
            await this.store.fetch(true);
            this.store.closeModal();
        } catch (e) {
            this.showValidation('error', e.message);
        } finally {
            this.submitting = false;
        }
    },

    async approve() {
        await this.store.approve(this.store.modalData.schedule.id);
    },

    async reject() {
        await this.store.reject(this.store.modalData.schedule.id);
    },

    async confirmDelete() {
        if (confirm('Delete this schedule?')) {
            await this.store.remove(this.store.modalData.schedule.id);
        }
    },

    isoToUtcInput(value) {
        const d = new Date(value);
        if (isNaN(d.getTime())) return '';
        const pad = (n) => n.toString().padStart(2, '0');
        return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}T${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}:${pad(d.getUTCSeconds())}`;
    },

    utcInputToIso(value) {
        if (!value) return '';
        // Parse datetime-local format as UTC: YYYY-MM-DDTHH:mm:ss
        const match = value.match(/^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})$/);
        if (!match) return '';
        const [_, year, month, day, hours, minutes, seconds] = match;
        const d = new Date(Date.UTC(year, month - 1, day, hours, minutes, seconds));
        return isNaN(d.getTime()) ? '' : d.toISOString();
    },

    formatDisplayDate(value) {
        return formatDateTime(value);
    },

    formatLocalTime(value) {
        if (!value) return '';
        const d = new Date(value);
        if (isNaN(d.getTime())) return '';
        const pad = (n) => n.toString().padStart(2, '0');

        // Get timezone offset in hours and minutes
        const offsetMinutes = -d.getTimezoneOffset();
        const offsetHours = Math.floor(Math.abs(offsetMinutes) / 60);
        const offsetMins = Math.abs(offsetMinutes) % 60;
        const offsetSign = offsetMinutes >= 0 ? '+' : '-';
        const offset = `UTC${offsetSign}${offsetHours}${offsetMins > 0 ? ':' + pad(offsetMins) : ''}`;

        const localTime = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
        return `${localTime} (${offset})`;
    },
});

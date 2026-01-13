// Schedule Editor component
export default () => ({
    variables: {},
    variablesExpanded: true,
    variablesStatus: 'Awaiting file...',
    validationState: 'idle',
    validationMessage: '',
    submitting: false,
    stepsEditor: null,
    validationTimer: null,

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
        this.$nextTick(() => {
            this.initCodeMirror();
            this.loadTemplate();
        });
    },

    initCodeMirror() {
        if (!window.CodeMirror) return;
        const container = this.$refs.stepsEditor;
        if (!container) return;

        this.stepsEditor = new window.CodeMirror.EditorView({
            doc: '',
            extensions: [
                window.CodeMirror.basicSetup,
                window.CodeMirror.yaml(),
                window.CodeMirror.oneDark,
                window.CodeMirror.EditorView.updateListener.of((update) => {
                    if (update.docChanged) this.scheduleValidation();
                }),
            ],
            parent: container,
        });
    },

    loadTemplate() {
        const now = new Date();
        const start = new Date(now.getTime() + 3600000);
        const end = new Date(start.getTime() + 900000);

        this.populateFromDocument({
            variables: { start: start.toISOString(), end: end.toISOString(), satellite: 'ISS' },
            steps: [{ time: 'T+0s', tracker: { observe: { target: '$satellite' } } }],
        });
        this.variablesStatus = 'Template loaded';
    },

    async handleFileUpload(event) {
        const file = event.target.files?.[0];
        if (!file) return;
        try {
            const doc = window.jsyaml.load(await file.text());
            this.populateFromDocument(doc);
            this.variablesStatus = file.name;
        } catch (e) {
            this.showValidation('error', 'Failed to parse file.');
        }
    },

    populateFromDocument(doc) {
        if (!doc || typeof doc !== 'object') return;

        const variables = { ...(doc.variables || {}) };
        if (!variables.start) variables.start = new Date(Date.now() + 3600000).toISOString();
        if (!variables.end) variables.end = new Date(new Date(variables.start).getTime() + 900000).toISOString();
        this.variables = variables;

        if (this.stepsEditor) {
            const yaml = window.jsyaml.dump(doc.steps || [], { lineWidth: 100 });
            this.stepsEditor.dispatch({
                changes: { from: 0, to: this.stepsEditor.state.doc.length, insert: yaml.trim() || '- {}' },
            });
        }
        this.scheduleValidation();
    },

    isDatetimeVar(name) { return name === 'start' || name === 'end'; },
    isMultilineVar(name) { return typeof this.variables[name] === 'string' && this.variables[name].includes('\n'); },

    getVariableInputValue(name) {
        const value = this.variables[name];
        return this.isDatetimeVar(name) ? this.isoToLocalInput(value) : value ?? '';
    },

    onVariableInput(name, event) {
        if (this.isDatetimeVar(name)) {
            this.variables[name] = this.localInputToIso(event.target.value) || '';
        } else {
            this.variables[name] = event.target.value;
        }
        this.scheduleValidation();
    },

    applyPreset(name, offsetMinutes) {
        const d = new Date();
        d.setMinutes(d.getMinutes() + offsetMinutes);
        this.variables[name] = d.toISOString();
        this.scheduleValidation();
    },

    toggleVariables() { this.variablesExpanded = !this.variablesExpanded; },

    scheduleValidation() {
        clearTimeout(this.validationTimer);
        this.validationTimer = setTimeout(() => this.validate(), 400);
    },

    buildScheduleYaml() {
        const stepsText = this.stepsEditor?.state.doc.toString() || '';
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

    async submit() {
        const auth = Alpine.store('auth');
        if (!auth.hasKey()) { auth.showModal(); return; }

        let yaml;
        try { yaml = this.buildScheduleYaml(); }
        catch (e) { this.showValidation('error', e.message); return; }

        this.submitting = true;
        try {
            const res = await auth.fetch('/api/schedules', {
                method: 'POST',
                headers: { 'Content-Type': 'application/yaml' },
                body: yaml,
            });
            if (!res.ok) throw new Error('Submit failed');

            Alpine.store('schedules').loadedRange = null;
            await Alpine.store('schedules').fetch(true);
            Alpine.store('schedules').closeEditor();
        } catch (e) {
            this.showValidation('error', e.message);
        } finally {
            this.submitting = false;
        }
    },

    isoToLocalInput(value) {
        const d = new Date(value);
        if (isNaN(d.getTime())) return '';
        const pad = (n) => n.toString().padStart(2, '0');
        return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
    },

    localInputToIso(value) {
        const d = new Date(value);
        return isNaN(d.getTime()) ? '' : d.toISOString();
    },

    formatDisplayDate(value) {
        const d = new Date(value);
        return isNaN(d.getTime()) ? value : d.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
    },
});

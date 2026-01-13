const ScheduleEditor = {
    elements: {
        fileInput: null,
        templateBtn: null,
        variablesCard: null,
        variablesBody: null,
        variablesStatus: null,
        variablesToggle: null,
        validationPanel: null,
        validationIcon: null,
        validationMessage: null,
        validationRange: null,
        schedulePreview: null,
        submitBtn: null,
    },
    stepsEditor: null,
    state: {
        variables: {},
        validationTimer: null,
        lastValidation: null,
    },

    init() {
        this.cacheElements();
        this.initEditor();
        this.bindEvents();
        this.loadTemplateDocument();
    },

    cacheElements() {
        this.elements.fileInput = document.getElementById('schedule-file-input');
        this.elements.templateBtn = document.getElementById('schedule-template-btn');
        this.elements.variablesCard = document.getElementById('variables-card');
        this.elements.variablesBody = document.getElementById('variables-body');
        this.elements.variablesStatus = document.getElementById('variables-status');
        this.elements.variablesToggle = document.getElementById('variables-toggle');
        this.elements.variablesTitle = document.getElementById('variables-title');
        this.elements.variablesDescription = document.getElementById('variables-description');
        this.elements.validationPanel = document.getElementById('validation-panel');
        this.elements.validationIcon = document.getElementById('validation-icon');
        this.elements.validationMessage = document.getElementById('validation-message');
        this.elements.validationRange = document.getElementById('validation-range');
        this.elements.schedulePreview = document.getElementById('schedule-preview');
        this.elements.submitBtn = document.getElementById('submit-schedule-btn');
    },

    initEditor() {
        if (!window.CodeMirror) return;
        this.stepsEditor = new CodeMirror.EditorView({
            doc: '',
            extensions: [
                CodeMirror.basicSetup,
                CodeMirror.yaml(),
                CodeMirror.oneDark,
                CodeMirror.EditorView.updateListener.of((update) => {
                    if (update.docChanged) {
                        this.scheduleValidationDebounced();
                    }
                }),
            ],
            parent: document.getElementById('steps-editor'),
        });
        this.updateStepsHeight();
    },

    bindEvents() {
        this.elements.fileInput?.addEventListener('change', (e) =>
            this.handleFileUpload(e.target.files?.[0] || null),
        );
        this.elements.templateBtn?.addEventListener('click', (e) => {
            e.preventDefault();
            this.loadTemplateDocument();
        });
        this.elements.variablesToggle?.addEventListener('click', (e) => {
            e.preventDefault();
            this.toggleVariablesCard();
        });
        this.elements.submitBtn?.addEventListener('click', (e) => {
            e.preventDefault();
            this.submitSchedule();
        });
    },

    loadTemplateDocument() {
        const now = new Date();
        const start = new Date(now.getTime() + 60 * 60 * 1000);
        const end = new Date(start.getTime() + 15 * 60 * 1000);
        const doc = {
            variables: {
                start: start.toISOString(),
                end: end.toISOString(),
                satellite: 'ISS',
            },
            steps: [
                {
                    time: 'T+0s',
                    tracker: {
                        observe: {
                            target: '$satellite',
                        },
                    },
                },
            ],
        };
        this.populateFromDocument(doc);
        this.elements.variablesStatus.textContent = 'Template loaded';
    },

    async handleFileUpload(file) {
        if (!file) {
            this.showValidation('info', 'No file selected.');
            return;
        }
        try {
            const text = await file.text();
            const doc = window.jsyaml.load(text);
            this.populateFromDocument(doc);
            this.elements.variablesStatus.textContent = file.name;
        } catch (error) {
            console.error(error);
            this.showValidation('error', 'Failed to parse schedule file.');
        }
    },

    populateFromDocument(doc) {
        if (!doc || typeof doc !== 'object') {
            this.showValidation('error', 'Invalid schedule document.');
            this.hideVariablesCard();
            return;
        }
        const variables = { ...(doc.variables || {}) };
        if (!variables.start) {
            const now = new Date();
            variables.start = new Date(now.getTime() + 3600000).toISOString();
        }
        if (!variables.end) {
            const start = new Date(variables.start);
            variables.end = new Date(start.getTime() + 900000).toISOString();
        }
        this.state.variables = variables;
        this.renderVariablesForm();

        if (this.stepsEditor) {
            const stepsYaml = window.jsyaml.dump(doc.steps || [], { lineWidth: 100 });
            this.stepsEditor.dispatch({
                changes: {
                    from: 0,
                    to: this.stepsEditor.state.doc.length,
                    insert: stepsYaml.trim() ? stepsYaml : '- {}',
                },
            });
            this.updateStepsHeight();
        }
        this.scheduleValidationDebounced();
    },

    renderVariablesForm() {
        const body = this.elements.variablesBody;
        const card = this.elements.variablesCard;
        if (!body || !card) return;
        body.innerHTML = '';
        card.classList.remove('hidden');
        card.dataset.expanded = card.dataset.expanded ?? 'true';
        if (card.dataset.expanded !== 'false') {
            body.classList.remove('hidden');
        } else {
            body.classList.add('hidden');
        }
        this.updateVariablesHeader();

        const entries = Object.entries(this.state.variables);
        const order = ['start', 'end'];
        const sorted = [
            ...order.filter((k) => k in this.state.variables).map((key) => [key, this.state.variables[key]]),
            ...entries.filter(([key]) => !order.includes(key)).sort(([a], [b]) => a.localeCompare(b)),
        ];

        for (const [name, value] of sorted) {
            const wrapper = document.createElement('div');
            wrapper.className = 'flex flex-col gap-1';

            const label = document.createElement('label');
            label.className = 'text-xs font-medium text-zinc-400';
            label.textContent = name;

            const useTextarea = typeof value === 'string' && value.includes('\n');
            const input = useTextarea ? document.createElement('textarea') : document.createElement('input');
            input.className =
                'w-full px-3 py-2 text-sm rounded border border-zinc-700 bg-zinc-900 text-zinc-100 focus:border-cyan-500 focus:outline-none';
            input.dataset.variable = name;

            if (name === 'start' || name === 'end') {
                input.type = 'datetime-local';
                input.value = this.isoToLocalInput(value);
                wrapper.appendChild(label);
                wrapper.appendChild(input);
                wrapper.appendChild(this.renderPresetButtons(name, input));
                body.appendChild(wrapper);
                continue;
            } else {
                if (useTextarea) {
                    input.rows = Math.min(6, value.split('\n').length + 1);
                    input.style.resize = 'vertical';
                } else {
                    input.type = 'text';
                }
                input.value = value ?? '';
            }

            input.addEventListener('input', () => this.onVariableInput(name, input));
            wrapper.appendChild(label);
            wrapper.appendChild(input);
            body.appendChild(wrapper);
        }
    },

    toggleVariablesCard() {
        const card = this.elements.variablesCard;
        const body = this.elements.variablesBody;
        if (!card || !body) return;
        const expanded = card.dataset.expanded !== 'false';
        card.dataset.expanded = expanded ? 'false' : 'true';
        if (expanded) {
            body.classList.add('hidden');
        } else {
            body.classList.remove('hidden');
        }
        this.updateVariablesHeader();
    },

    hideVariablesCard() {
        const card = this.elements.variablesCard;
        if (card) {
            card.classList.add('hidden');
        }
    },

    updateVariablesHeader() {
        const card = this.elements.variablesCard;
        const toggle = this.elements.variablesToggle;
        const title = this.elements.variablesTitle;
        const description = this.elements.variablesDescription;
        if (!card || !title) return;
        const expanded = card.dataset.expanded !== 'false';
        if (toggle) {
            toggle.textContent = expanded ? '▾' : '▸';
            toggle.dataset.state = expanded ? 'expanded' : 'collapsed';
        }
        if (expanded) {
            title.textContent = 'Variables';
            description?.classList.remove('hidden');
        } else {
            title.textContent = 'Variables (v)';
            description?.classList.add('hidden');
        }
    },

    onVariableInput(name, input) {
        if (name === 'start' || name === 'end') {
            const iso = this.localInputToIso(input.value);
            this.state.variables[name] = iso || '';
        } else {
            this.state.variables[name] = input.value;
        }
        this.scheduleValidationDebounced();
    },

    renderPresetButtons(name, input) {
        const container = document.createElement('div');
        container.className = 'flex gap-2 mt-1 text-xs text-zinc-500';
        [
            { label: 'Now', offset: 0 },
            { label: 'Now +5m', offset: 5 },
            { label: 'Now +10m', offset: 10 },
        ].forEach((preset) => {
            const button = document.createElement('button');
            button.type = 'button';
            button.className = 'px-2 py-0.5 border border-zinc-800 rounded hover:border-cyan-500 hover:text-cyan-300 transition-colors';
            button.textContent = preset.label;
            button.addEventListener('click', () => {
                const now = new Date();
                now.setMinutes(now.getMinutes() + preset.offset);
                const iso = now.toISOString();
                this.state.variables[name] = iso;
                input.value = this.isoToLocalInput(iso);
                this.scheduleValidationDebounced();
            });
            container.appendChild(button);
        });
        return container;
    },

    updateStepsHeight() {
        if (!this.stepsEditor) return;
        const container = this.stepsEditor.dom;
        const lines = this.stepsEditor.state.doc.lines;
        const lineHeight = 22;
        const min = 220;
        const max = 1200;
        const height = Math.max(min, Math.min(max, lines * lineHeight + 60));
        container.style.height = `${height}px`;
    },

    scheduleValidationDebounced() {
        clearTimeout(this.state.validationTimer);
        this.state.validationTimer = setTimeout(() => this.validateSchedule(), 400);
    },

    buildScheduleYaml() {
        const variables = { ...this.state.variables };
        const stepsText = this.stepsEditor ? this.stepsEditor.state.doc.toString() : '';
        let stepsAst = [];
        if (stepsText.trim()) {
            const parsed = window.jsyaml.load(stepsText);
            if (!Array.isArray(parsed)) {
                throw new Error('Steps must be a YAML list.');
            }
            stepsAst = parsed;
        }
        return window.jsyaml.dump(
            {
                variables,
                steps: stepsAst,
            },
            { lineWidth: 100 },
        );
    },

    async validateSchedule() {
        let yaml;
        try {
            yaml = this.buildScheduleYaml();
        } catch (error) {
            this.showValidation('error', error.message);
            this.elements.submitBtn.disabled = true;
            this.elements.schedulePreview.textContent = 'Invalid schedule.';
            return;
        }

        if (!Auth.hasKey()) {
            this.showValidation('info', 'Enter an API key to validate schedules.');
            this.elements.submitBtn.disabled = true;
            return;
        }

        this.showValidation('pending', 'Validating…');

        try {
            const response = await Auth.fetch('/api/schedules/validate', {
                method: 'POST',
                headers: { 'Content-Type': 'application/yaml' },
                body: yaml,
            });
            const data = await response.json();
            if (!response.ok) {
                throw new Error(data.message || data.error || 'Validation failed');
            }

            if (data.valid) {
                this.state.lastValidation = yaml;
                this.elements.submitBtn.disabled = false;
                const range =
                    data.start && data.end
                        ? `${this.formatDisplayDate(data.start)} → ${this.formatDisplayDate(data.end)}`
                        : '';
                this.showValidation('success', range || 'Schedule ready to submit.');
            } else {
                this.elements.submitBtn.disabled = true;
                const message = data.errors?.[0] || 'Validation failed.';
                this.showValidation('error', message);
            }
        } catch (error) {
            console.error(error);
            this.elements.submitBtn.disabled = true;
            this.showValidation('error', error.message || 'Validation failed.');
        }
    },

    async submitSchedule() {
        if (!Auth.hasKey()) {
            Auth.showModal();
            return;
        }
        let yaml;
        try {
            yaml = this.buildScheduleYaml();
        } catch (error) {
            this.showValidation('error', error.message);
            return;
        }

        const btn = this.elements.submitBtn;
        btn.disabled = true;
        const previous = btn.textContent;
        btn.textContent = 'Submitting…';

        try {
            const response = await Auth.fetch('/api/schedules', {
                method: 'POST',
                headers: { 'Content-Type': 'application/yaml' },
                body: yaml,
            });
            if (!response.ok) {
                const data = await response.json().catch(() => ({}));
                throw new Error(data.message || data.error || 'Failed to submit schedule');
            }
            document.body.dispatchEvent(new CustomEvent('scheduleUpdated'));
            this.showValidation('success', 'Schedule submitted.');
            closeNewScheduleModal();
        } catch (error) {
            this.showValidation('error', error.message || 'Submission failed.');
        } finally {
            btn.disabled = false;
            btn.textContent = previous;
        }
    },

    showValidation(state, message) {
        const preview = this.elements.schedulePreview;
        if (!preview) return;
        const prefix = {
            success: 'Schedule valid:',
            error: 'Validation error:',
            info: '',
            pending: 'Validating:',
        }[state] || '';
        preview.classList.remove('text-zinc-400', 'text-green-400', 'text-rose-400', 'text-amber-400');
        const color =
            state === 'success'
                ? 'text-green-400'
                : state === 'error'
                ? 'text-rose-400'
                : state === 'pending'
                ? 'text-amber-400'
                : 'text-zinc-400';
        preview.classList.add(color);
        preview.textContent = prefix ? `${prefix} ${message}` : message || '';
    },

    isoToLocalInput(value) {
        const date = new Date(value);
        if (Number.isNaN(date.getTime())) return '';
        const pad = (n) => n.toString().padStart(2, '0');
        return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(
            date.getHours(),
        )}:${pad(date.getMinutes())}`;
    },

    localInputToIso(value) {
        if (!value) return '';
        const date = new Date(value);
        if (Number.isNaN(date.getTime())) return '';
        return date.toISOString();
    },

    formatDisplayDate(value) {
        const date = new Date(value);
        if (Number.isNaN(date.getTime())) return value;
        return date.toLocaleString(undefined, {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        });
    },
};

window.ScheduleEditor = ScheduleEditor;
document.addEventListener('DOMContentLoaded', () => ScheduleEditor.init());

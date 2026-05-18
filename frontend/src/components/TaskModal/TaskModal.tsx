import { useState, useEffect, useCallback, useRef } from 'react';
import { X, Plus, Trash2 } from 'lucide-react';
import { getTask, putTask, deleteTask } from '../../api/tasks';
import { listTemplates, getTemplate, submitFromTemplate } from '../../api/templates';
import type { TemplateListEntry } from '../../api/templates';
import type { ApiPass } from '../../api/types';
import styles from './TaskModal.module.css';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface Variable {
  name: string;
  value: string;
}

function parseVariablesFromYaml(yaml: string): Variable[] {
  const vars: Variable[] = [];
  const lines = yaml.split('\n');
  let inVars = false;
  for (const line of lines) {
    if (/^variables:\s*$/.test(line)) { inVars = true; continue; }
    if (inVars) {
      if (/^\S/.test(line) && line.trim() !== '') break;
      const m = line.match(/^\s+(\w+):\s*"(.*)"\s*$/) ?? line.match(/^\s+(\w+):\s*(.*)\s*$/);
      if (m) vars.push({ name: m[1], value: m[2] });
    }
  }
  return vars;
}

function buildVariablesBlock(vars: Variable[]): string {
  const valid = vars.filter((v) => v.name.trim() !== '');
  if (valid.length === 0) return '';
  return 'variables:\n' + valid.map((v) => `  ${v.name}: "${v.value}"`).join('\n');
}

function replaceVariablesInYaml(yaml: string, vars: Variable[]): string {
  const lines = yaml.split('\n');
  const newBlock = buildVariablesBlock(vars);
  let varStart = -1;
  let varEnd = lines.length;
  for (let i = 0; i < lines.length; i++) {
    if (/^variables:\s*$/.test(lines[i])) {
      varStart = i;
      for (let j = i + 1; j < lines.length; j++) {
        if (/^\S/.test(lines[j]) && lines[j].trim() !== '') { varEnd = j; break; }
      }
      break;
    }
  }
  if (varStart >= 0) {
    return [...lines.slice(0, varStart), newBlock, ...lines.slice(varEnd)].join('\n');
  }
  return newBlock ? newBlock + '\n' + yaml : yaml;
}

function isTimeVariable(name: string): boolean {
  return name === 'start' || name === 'end';
}

function isoToDatetimeLocal(iso: string): string {
  try {
    const d = new Date(iso);
    return isNaN(d.getTime()) ? '' : d.toISOString().slice(0, 16);
  } catch { return ''; }
}

function datetimeLocalToIso(val: string): string {
  return val ? new Date(val + 'Z').toISOString() : '';
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface PassInfo {
  satellite: string;
  pass: ApiPass;
}

type Mode =
  | { kind: 'edit'; taskId: string }
  | { kind: 'create'; fromPass?: PassInfo };

export type { Mode as TaskModalMode };

// ---------------------------------------------------------------------------
// TaskModal
// ---------------------------------------------------------------------------

interface TaskModalProps {
  mode: Mode;
  onClose: () => void;
  onSaved: () => void;
}

export function TaskModal({ mode, onClose, onSaved }: TaskModalProps) {
  const isEdit = mode.kind === 'edit';
  const fromPass = 'fromPass' in mode ? mode.fromPass : undefined;

  const [taskId, setTaskId] = useState(isEdit ? mode.taskId : '');
  const [yaml, setYaml] = useState(isEdit ? (null as string | null) : '');
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [variables, setVariables] = useState<Variable[]>(() =>
    fromPass
      ? [{ name: 'start', value: fromPass.pass.start }, { name: 'end', value: fromPass.pass.end }]
      : [],
  );

  const [templates, setTemplates] = useState<TemplateListEntry[]>([]);
  const [selectedTemplateId, setSelectedTemplateId] = useState<string | null>(null);
  const [templateYaml, setTemplateYaml] = useState<string | null>(null);
  const [templateLoading, setTemplateLoading] = useState(false);

  // When true, the next YAML onChange was triggered by us, skip re-parsing vars
  const suppressRef = useRef(false);
  const editorRef = useRef<HTMLTextAreaElement>(null);

  const usingTemplate = selectedTemplateId != null && templateYaml != null;

  // Load existing task
  useEffect(() => {
    if (!isEdit) return;
    let cancelled = false;
    getTask(mode.taskId)
      .then((text) => { if (!cancelled) { setYaml(text); setVariables(parseVariablesFromYaml(text)); } })
      .catch((err) => { if (!cancelled) setError(String(err)); });
    return () => { cancelled = true; };
  }, [mode, isEdit]);

  // Load template list
  useEffect(() => {
    if (isEdit) return;
    listTemplates().then(setTemplates).catch(() => {});
  }, [isEdit]);

  // Initialise task ID from pass
  useEffect(() => {
    if (fromPass && !taskId) setTaskId(`${fromPass.satellite}-pass`);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Initialise YAML skeleton when creating from pass
  useEffect(() => {
    if (fromPass && yaml === '') {
      const block = buildVariablesBlock(variables);
      setYaml(block ? block + '\nsteps:\n  - cmd: ""\n' : '');
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Template select
  const handleTemplateSelect = useCallback(async (templateId: string) => {
    if (!templateId) {
      setSelectedTemplateId(null);
      setTemplateYaml(null);
      return;
    }
    setSelectedTemplateId(templateId);
    setTemplateLoading(true);
    setError(null);
    try {
      const text = await getTemplate(templateId);
      setTemplateYaml(text);
      const tplVars = parseVariablesFromYaml(text);
      const mergedVars = (prev: Variable[]) => {
        const prevMap = new Map(prev.map((v) => [v.name, v.value]));
        return tplVars.map((tv) => ({ name: tv.name, value: prevMap.get(tv.name) || tv.value }));
      };
      setVariables((prev) => {
        const merged = mergedVars(prev);
        // Set the YAML preview with merged variable values applied
        suppressRef.current = true;
        setYaml(replaceVariablesInYaml(text, merged));
        return merged;
      });
    } catch (err) {
      setError(String(err));
      setSelectedTemplateId(null);
      setTemplateYaml(null);
    } finally {
      setTemplateLoading(false);
    }
  }, []);

  // Push variable changes into the YAML editor
  const pushVarsToYaml = useCallback((vars: Variable[]) => {
    suppressRef.current = true;
    if (templateYaml) {
      // Always rebase on the original template to avoid drift
      setYaml(replaceVariablesInYaml(templateYaml, vars));
    } else {
      setYaml((prev) => replaceVariablesInYaml(prev ?? '', vars));
    }
  }, [templateYaml]);

  // YAML editor change — re-parse vars unless we triggered it
  const handleYamlChange = useCallback((text: string) => {
    setYaml(text);
    if (suppressRef.current) { suppressRef.current = false; return; }
    setVariables(parseVariablesFromYaml(text));
  }, []);

  const handleVarNameChange = useCallback((i: number, name: string) => {
    setVariables((prev) => {
      const next = prev.map((v, idx) => idx === i ? { ...v, name } : v);
      pushVarsToYaml(next);
      return next;
    });
  }, [pushVarsToYaml]);

  const handleVarValueChange = useCallback((i: number, value: string) => {
    setVariables((prev) => {
      const next = prev.map((v, idx) => idx === i ? { ...v, value } : v);
      pushVarsToYaml(next);
      return next;
    });
  }, [pushVarsToYaml]);

  const handleAddVariable = useCallback(() => {
    setVariables((prev) => {
      const next = [...prev, { name: '', value: '' }];
      pushVarsToYaml(next);
      return next;
    });
  }, [pushVarsToYaml]);

  const handleRemoveVariable = useCallback((i: number) => {
    setVariables((prev) => {
      const next = prev.filter((_, idx) => idx !== i);
      pushVarsToYaml(next);
      return next;
    });
  }, [pushVarsToYaml]);

  // Keyboard / backdrop
  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => { if (e.target === e.currentTarget) onClose(); },
    [onClose],
  );
  useEffect(() => {
    const h = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose(); };
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, [onClose]);

  // Tab key in YAML editor
  const handleTab = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key !== 'Tab') return;
    e.preventDefault();
    const ta = e.currentTarget;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    if (!e.shiftKey) {
      const next = ta.value.slice(0, start) + '  ' + ta.value.slice(end);
      handleYamlChange(next);
      requestAnimationFrame(() => { ta.selectionStart = ta.selectionEnd = start + 2; });
    } else {
      const lineStart = ta.value.lastIndexOf('\n', start - 1) + 1;
      const spaces = ta.value.slice(lineStart).match(/^ {1,2}/)?.[0].length ?? 0;
      if (spaces > 0) {
        handleYamlChange(ta.value.slice(0, lineStart) + ta.value.slice(lineStart + spaces));
        requestAnimationFrame(() => { ta.selectionStart = ta.selectionEnd = Math.max(lineStart, start - spaces); });
      }
    }
  }, [handleYamlChange]);

  // Save
  const handleSave = useCallback(async () => {
    const id = taskId.trim();
    if (!id) { setError('Task ID is required'); return; }
    setSaving(true);
    setError(null);
    try {
      if (usingTemplate && selectedTemplateId) {
        const varsMap: Record<string, string> = {};
        for (const v of variables) { if (v.name.trim()) varsMap[v.name.trim()] = v.value; }
        await submitFromTemplate({ template_id: selectedTemplateId, task_id: id, variables: varsMap });
      } else {
        if (!yaml?.trim()) { setError('YAML body is required'); setSaving(false); return; }
        await putTask(id, yaml);
      }
      onSaved();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }, [taskId, yaml, usingTemplate, selectedTemplateId, variables, onSaved]);

  // Delete
  const handleDelete = useCallback(async () => {
    if (!isEdit) return;
    if (!confirm(`Delete task "${mode.taskId}"?`)) return;
    setSaving(true);
    setError(null);
    try {
      await deleteTask(mode.taskId);
      onSaved();
    } catch (err) {
      setError(String(err));
      setSaving(false);
    }
  }, [mode, isEdit, onSaved]);

  const loading = isEdit && yaml === null && !error;

  return (
    <div className={styles.backdrop} onClick={handleBackdropClick}>
      <div className={styles.modal}>
        {/* Header */}
        <div className={styles.header}>
          {isEdit ? (
            <span className={styles.title}>{taskId}</span>
          ) : (
            <input
              className={styles.titleInput}
              value={taskId}
              onChange={(e) => setTaskId(e.target.value)}
              placeholder="Task ID"
              autoFocus
              spellCheck={false}
            />
          )}
          <button className={styles.closeButton} onClick={onClose} title="Close">
            <X size={16} />
          </button>
        </div>

        {/* Body */}
        <div className={styles.body}>
          {error && <div className={styles.error}>{error}</div>}
          {loading ? (
            <div className={styles.loading}>Loading...</div>
          ) : (
            <>
              {/* Variables section */}
              <div className={styles.varsSection}>
                {/* Template selector (create mode only) */}
                {!isEdit && (
                  <div className={styles.templateRow}>
                    <span className={styles.sectionLabel}>Template</span>
                    <select
                      className={styles.templateSelect}
                      value={selectedTemplateId ?? ''}
                      onChange={(e) => handleTemplateSelect(e.target.value)}
                      disabled={templateLoading}
                    >
                      <option value="">None</option>
                      {templates.map((t) => (
                        <option key={t.id} value={t.id}>{t.id}</option>
                      ))}
                    </select>
                    {templateLoading && <span className={styles.hint}>Loading...</span>}
                  </div>
                )}

                {/* Variable rows */}
                <div className={styles.varsHeader}>
                  <span className={styles.sectionLabel}>Variables</span>
                  {!usingTemplate && (
                    <button className={styles.addBtn} onClick={handleAddVariable} title="Add variable">
                      <Plus size={12} />
                    </button>
                  )}
                </div>
                {variables.length === 0 ? (
                  <div className={styles.emptyVars}>
                    {usingTemplate ? 'No variables in template' : 'No variables'}
                  </div>
                ) : (
                  <div className={styles.varsGrid}>
                    {variables.map((v, i) => (
                      <VariableRow
                        key={i}
                        variable={v}
                        nameReadOnly={usingTemplate}
                        onNameChange={(name) => handleVarNameChange(i, name)}
                        onValueChange={(val) => handleVarValueChange(i, val)}
                        onRemove={!usingTemplate ? () => handleRemoveVariable(i) : undefined}
                      />
                    ))}
                  </div>
                )}
              </div>

              {/* YAML editor */}
              <textarea
                ref={editorRef}
                className={styles.editor}
                value={yaml ?? ''}
                onChange={(e) => handleYamlChange(e.target.value)}
                onKeyDown={handleTab}
                readOnly={usingTemplate}
                spellCheck={false}
                wrap="off"
              />
            </>
          )}
        </div>

        {/* Footer */}
        <div className={styles.footer}>
          {isEdit && (
            <button
              className={`${styles.btn} ${styles.btnDanger}`}
              onClick={handleDelete}
              disabled={saving || loading}
            >
              Delete
            </button>
          )}
          <div className={styles.spacer} />
          <button
            className={`${styles.btn} ${styles.btnPrimary}`}
            onClick={handleSave}
            disabled={saving || loading}
          >
            {saving ? 'Saving...' : isEdit ? 'Update' : usingTemplate ? 'Submit' : 'Create'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// VariableRow
// ---------------------------------------------------------------------------

interface VariableRowProps {
  variable: Variable;
  nameReadOnly: boolean;
  onNameChange: (name: string) => void;
  onValueChange: (value: string) => void;
  onRemove?: () => void;
}

function VariableRow({ variable, nameReadOnly, onNameChange, onValueChange, onRemove }: VariableRowProps) {
  const { name, value } = variable;
  const isTime = isTimeVariable(name);
  return (
    <div className={styles.varRow}>
      <div className={styles.varName}>
        {nameReadOnly ? (
          <span className={styles.varNameLabel}>
            {name}{isTime && <span className={styles.varTag}>time</span>}
          </span>
        ) : (
          <input
            className={styles.varNameInput}
            type="text"
            value={name}
            onChange={(e) => onNameChange(e.target.value)}
            placeholder="name"
            spellCheck={false}
          />
        )}
      </div>
      <div className={styles.varValue}>
        {isTime
          ? <TimeInput value={value} onChange={onValueChange} />
          : <input className={styles.varInput} type="text" value={value} onChange={(e) => onValueChange(e.target.value)} placeholder="value" spellCheck={false} />
        }
      </div>
      {onRemove && (
        <button className={styles.removeBtn} onClick={onRemove} title="Remove">
          <Trash2 size={12} />
        </button>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// TimeInput
// ---------------------------------------------------------------------------

function TimeInput({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const isRelative = /^[T$]/.test(value) || /^[+-]/.test(value);
  const [mode, setMode] = useState<'absolute' | 'relative'>(isRelative ? 'relative' : 'absolute');
  return (
    <div className={styles.timeGroup}>
      <select className={styles.timeMode} value={mode} onChange={(e) => setMode(e.target.value as 'absolute' | 'relative')}>
        <option value="absolute">UTC</option>
        <option value="relative">Relative</option>
      </select>
      {mode === 'absolute' ? (
        <input className={styles.varInput} type="datetime-local" value={isoToDatetimeLocal(value)} onChange={(e) => onChange(datetimeLocalToIso(e.target.value))} />
      ) : (
        <input className={styles.varInput} type="text" value={value} onChange={(e) => onChange(e.target.value)} placeholder="e.g. T+30s, $end-1 minute" spellCheck={false} />
      )}
    </div>
  );
}

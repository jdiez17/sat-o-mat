import { apiFetch } from './client';

export interface TemplateListEntry {
  id: string;
}

export interface SubmitFromTemplateRequest {
  template_id: string;
  task_id: string;
  variables: Record<string, string>;
}

export async function listTemplates(): Promise<TemplateListEntry[]> {
  const res = await apiFetch('/api/templates');
  if (!res.ok) throw new Error(`Failed to list templates: ${res.status}`);
  return res.json();
}

export async function getTemplate(id: string): Promise<string> {
  const res = await apiFetch(`/api/templates/${encodeURIComponent(id)}`);
  if (!res.ok) throw new Error(`Failed to get template: ${res.status}`);
  return res.text();
}

export async function submitFromTemplate(req: SubmitFromTemplateRequest): Promise<void> {
  const res = await apiFetch('/api/tasks/submit_from_template', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `Failed to submit task: ${res.status}`);
  }
}

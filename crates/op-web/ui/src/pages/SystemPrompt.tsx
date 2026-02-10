import { useEffect, useMemo, useState } from 'react';
import { FileText } from 'lucide-react';
import { apiGet, apiPost } from '@/lib/backend';

interface SystemPromptResponse {
  full_prompt: string;
  fixed_part: string;
  custom_part: string;
  custom_source: string;
  char_count: number;
  estimated_tokens: number;
}

interface SaveResponse {
  success: boolean;
  message: string;
}

export default function SystemPrompt() {
  const [immutable, setImmutable] = useState('');
  const [tunable, setTunable] = useState('');
  const [fullPrompt, setFullPrompt] = useState('');
  const [loading, setLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [saveStatus, setSaveStatus] = useState<string | null>(null);

  const loadPrompt = async () => {
    setLoading(true);
    try {
      const data = await apiGet<SystemPromptResponse>('/admin/prompt');
      setImmutable(data.fixed_part || '');
      setTunable(data.custom_part || '');
      setFullPrompt(data.full_prompt || '');
      setSaveStatus(null);
    } catch (err) {
      setSaveStatus(err instanceof Error ? err.message : 'Failed to load prompt');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPrompt();
  }, []);

  const handleSave = async () => {
    setIsSaving(true);
    setSaveStatus(null);
    try {
      const res = await apiPost<SaveResponse>('/admin/prompt/custom', { content: tunable });
      await apiPost('/admin/prompt/reload', {});
      await loadPrompt();
      setSaveStatus(res.success ? 'Saved' : res.message || 'Save failed');
    } catch (err) {
      setSaveStatus(err instanceof Error ? err.message : 'Save failed');
    } finally {
      setIsSaving(false);
    }
  };

  const preview = useMemo(() => {
    if (fullPrompt) return fullPrompt;
    return `${immutable}\n\n---\n\n${tunable}`;
  }, [fullPrompt, immutable, tunable]);

  return (
    <div className="h-full flex flex-col bg-background">
      <div className="px-6 py-4 border-b border-border bg-card/50">
        <h2 className="text-lg font-bold text-foreground flex items-center gap-3">
          <FileText className="h-5 w-5 text-primary" />
          System Prompt Configuration
        </h2>
        <p className="text-xs text-muted-foreground mt-1">
          Live admin prompt editor backed by `op-web` routes
        </p>
      </div>

      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {loading ? (
          <div className="text-sm text-muted-foreground">Loading system prompt...</div>
        ) : (
          <>
            <div className="bg-card border border-border rounded-lg overflow-hidden">
              <div className="px-4 py-3 border-b border-border bg-background/50 flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-destructive" />
                  <h3 className="text-sm font-semibold text-foreground">Immutable Core</h3>
                </div>
                <span className="text-[10px] bg-destructive/20 text-destructive px-2 py-0.5 rounded border border-destructive/30">
                  READ-ONLY
                </span>
              </div>
              <div className="p-4">
                <pre className="bg-background border border-border rounded p-4 text-xs font-mono text-foreground/80 whitespace-pre-wrap overflow-x-auto">
                  {immutable}
                </pre>
              </div>
            </div>

            <div className="bg-card border border-border rounded-lg overflow-hidden">
              <div className="px-4 py-3 border-b border-border bg-background/50 flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-success" />
                  <h3 className="text-sm font-semibold text-foreground">Tunable Context</h3>
                </div>
                <span className="text-[10px] bg-success/20 text-success px-2 py-0.5 rounded border border-success/30">
                  EDITABLE
                </span>
              </div>
              <div className="p-4">
                <textarea
                  className="w-full bg-background border border-border rounded p-4 text-xs font-mono text-foreground resize-none focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary"
                  rows={10}
                  value={tunable}
                  onChange={(e) => setTunable(e.target.value)}
                />
                <div className="mt-3 flex items-center justify-between">
                  <div className="text-xs text-muted-foreground">Changes are written to custom prompt storage.</div>
                  <div className="flex items-center gap-3">
                    {saveStatus && <span className="text-xs text-muted-foreground">{saveStatus}</span>}
                    <button
                      className="px-4 py-2 bg-primary hover:bg-primary/90 text-primary-foreground text-xs font-medium rounded transition-colors disabled:opacity-50"
                      disabled={isSaving}
                      onClick={handleSave}
                    >
                      {isSaving ? 'Saving...' : 'Save Changes'}
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <div className="bg-card border border-border rounded-lg overflow-hidden">
              <div className="px-4 py-3 border-b border-border bg-background/50">
                <h3 className="text-sm font-semibold text-foreground">Combined Prompt Preview</h3>
              </div>
              <div className="p-4">
                <div className="bg-background border border-border rounded p-4 text-xs font-mono text-muted-foreground whitespace-pre-wrap max-h-48 overflow-y-auto">
                  {preview}
                </div>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

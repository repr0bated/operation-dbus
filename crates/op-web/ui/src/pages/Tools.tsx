import { useEffect, useMemo, useState } from 'react';
import { Wrench, Search, Filter } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { apiGet } from '@/lib/backend';

interface ToolItem {
  name: string;
  category: string;
  description: string;
}

interface ToolsResponse {
  tools: ToolItem[];
  count: number;
}

export default function Tools() {
  const [searchQuery, setSearchQuery] = useState('');
  const [tools, setTools] = useState<ToolItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadTools = async () => {
    setLoading(true);
    try {
      const data = await apiGet<ToolsResponse>('/api/tools');
      setTools(data.tools || []);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load tools');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadTools();
  }, []);

  const filteredTools = useMemo(
    () => tools.filter((tool) =>
      tool.name.toLowerCase().includes(searchQuery.toLowerCase())
      || tool.category.toLowerCase().includes(searchQuery.toLowerCase())
      || tool.description.toLowerCase().includes(searchQuery.toLowerCase()),
    ),
    [tools, searchQuery],
  );

  return (
    <div className="p-8 max-w-[1600px]">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Tool Registry</h1>
        <p className="text-muted-foreground">Live tools from backend registry</p>
      </div>

      <Card className="mb-6">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Search Tools</CardTitle>
              <CardDescription>Find and explore available system tools</CardDescription>
            </div>
            <div className="flex items-center gap-2">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search tools..."
                  className="pl-9 w-64"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                />
              </div>
              <Button variant="outline" size="icon" onClick={loadTools}>
                <Filter className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </CardHeader>
      </Card>

      {loading ? (
        <div className="text-sm text-muted-foreground">Loading tools...</div>
      ) : error ? (
        <div className="text-sm text-destructive">{error}</div>
      ) : (
        <>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {filteredTools.map((tool) => (
              <Card key={tool.name} className="hover:border-primary transition-colors cursor-pointer">
                <CardHeader className="pb-2">
                  <div className="flex items-center gap-2">
                    <Wrench className="h-4 w-4 text-primary" />
                    <CardTitle className="text-base font-mono">{tool.name}</CardTitle>
                  </div>
                  <span className="text-xs px-2 py-1 rounded-full bg-primary/10 text-primary w-fit">
                    {tool.category}
                  </span>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground">{tool.description}</p>
                </CardContent>
              </Card>
            ))}
          </div>

          <div className="mt-8 text-center py-8 text-muted-foreground">
            <p className="text-sm">Showing {filteredTools.length} of {tools.length} tools</p>
          </div>
        </>
      )}
    </div>
  );
}

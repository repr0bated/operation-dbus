import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { ApiProvider } from '@/api/provider';
import { AppShell, ChatPanel } from '@/components';
import { DashboardPage, ToolsPage, AgentsPage, DbusPage, WorkflowsPage, StatePage } from '@/pages';

export default function App() {
  return (
    <ApiProvider>
      <BrowserRouter>
        <AppShell>
          <Routes>
            <Route path="/" element={<DashboardPage />} />
            <Route path="/chat" element={<div className="h-full"><ChatPanel /></div>} />
            <Route path="/tools" element={<ToolsPage />} />
            <Route path="/agents" element={<AgentsPage />} />
            <Route path="/dbus" element={<DbusPage />} />
            <Route path="/workflows" element={<WorkflowsPage />} />
            <Route path="/state" element={<StatePage />} />
          </Routes>
        </AppShell>
      </BrowserRouter>
    </ApiProvider>
  );
}

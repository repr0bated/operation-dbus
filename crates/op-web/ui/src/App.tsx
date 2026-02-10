import { Toaster } from "@/components/ui/toaster";
import { Toaster as Sonner } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Layout } from "@/components/Layout";
import IntegratedDashboard from "./pages/IntegratedDashboard";
import SystemPrompt from "./pages/SystemPrompt";
import Tools from "./pages/Tools";
import McpExecution from "./pages/McpExecution";
import Plugins from "./pages/Plugins";
import Audit from "./pages/Audit";
import NotFound from "./pages/NotFound";

const queryClient = new QueryClient();

const App = () => (
  <QueryClientProvider client={queryClient}>
    <TooltipProvider>
      <Toaster />
      <Sonner />
      <BrowserRouter>
        <Layout>
          <Routes>
            <Route path="/" element={<IntegratedDashboard />} />
            <Route path="/system-prompt" element={<SystemPrompt />} />
            <Route path="/tools" element={<Tools />} />
            <Route path="/mcp-execution" element={<McpExecution />} />
            <Route path="/workflows" element={<Audit />} />
            <Route path="/mcp-groups" element={<Audit />} />
            <Route path="/plugins" element={<Plugins />} />
            <Route path="/logs" element={<Audit />} />
            <Route path="*" element={<NotFound />} />
          </Routes>
        </Layout>
      </BrowserRouter>
    </TooltipProvider>
  </QueryClientProvider>
);

export default App;

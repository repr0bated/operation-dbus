import { NavLink, useLocation } from "react-router-dom";
import {
  LayoutGrid,
  FileText,
  Wrench,
  Activity,
  Users,
  Settings,
  FileCode,
  Terminal,
} from "lucide-react";
import { cn } from "@/lib/utils";

interface NavItem {
  icon: React.ElementType;
  label: string;
  path: string;
}

const navItems: NavItem[] = [
  { icon: LayoutGrid, label: "Dashboard", path: "/" },
  { icon: FileText, label: "System Prompt", path: "/system-prompt" },
  { icon: Wrench, label: "Tools", path: "/tools" },
  { icon: Terminal, label: "MCP Execution", path: "/mcp-execution" },
  { icon: Activity, label: "Workflows", path: "/workflows" },
  { icon: Users, label: "MCP Groups", path: "/mcp-groups" },
  { icon: Settings, label: "Plugins", path: "/plugins" },
  { icon: FileCode, label: "Logs", path: "/logs" },
];

interface IconSidebarProps {
  connectionStatus: "connected" | "connecting" | "disconnected";
}

export function IconSidebar({ connectionStatus }: IconSidebarProps) {
  const location = useLocation();

  return (
    <nav className="w-14 bg-sidebar border-r border-border flex flex-col items-center py-4 gap-2">
      {navItems.map((item) => {
        const isActive = location.pathname === item.path;
        const Icon = item.icon;

        return (
          <NavLink
            key={item.path}
            to={item.path}
            className={cn(
              "w-10 h-10 rounded-lg flex items-center justify-center transition-colors group relative",
              isActive
                ? "bg-accent text-accent-foreground"
                : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
            )}
            title={item.label}
          >
            <Icon className="h-[18px] w-[18px]" />
            <span className="absolute left-12 bg-popover text-popover-foreground text-xs px-2 py-1 rounded opacity-0 group-hover:opacity-100 pointer-events-none whitespace-nowrap z-50 border border-border shadow-md">
              {item.label}
            </span>
          </NavLink>
        );
      })}

      {/* Connection indicator at bottom */}
      <div className="mt-auto">
        <div
          className={cn(
            "w-3 h-3 rounded-full",
            connectionStatus === "connected" &&
              "bg-success shadow-[0_0_8px_hsl(var(--success)/0.5)]",
            connectionStatus === "connecting" &&
              "bg-warning animate-pulse",
            connectionStatus === "disconnected" && "bg-destructive"
          )}
        />
      </div>
    </nav>
  );
}

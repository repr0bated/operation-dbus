import { useState } from "react";
import { IconSidebar } from "@/components/IconSidebar";

interface LayoutProps {
  children: React.ReactNode;
}

export function Layout({ children }: LayoutProps) {
  const [connectionStatus] = useState<"connected" | "connecting" | "disconnected">("connected");

  return (
    <div className="h-screen flex w-full bg-background">
      <IconSidebar connectionStatus={connectionStatus} />
      <main className="flex-1 overflow-hidden">
        {children}
      </main>
    </div>
  );
}

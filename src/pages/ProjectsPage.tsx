import { PageEmpty } from "@/components/PageState";
import { FolderKanban } from "lucide-react";

export default function ProjectsPage() {
  return (
    <main className="flex-1 overflow-hidden">
      <PageEmpty
        icon={FolderKanban}
        title="Projects coming soon"
        message="Project status, milestones, and deliverables will live here."
      />
    </main>
  );
}

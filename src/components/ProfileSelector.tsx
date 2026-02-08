import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Building2, FolderKanban, Check } from "lucide-react";
import { cn } from "@/lib/utils";
import type { ProfileType } from "@/types";

interface ProfileOption {
  id: ProfileType;
  title: string;
  description: string;
  icon: typeof Building2;
  features: string[];
}

const profiles: ProfileOption[] = [
  {
    id: "customer-success",
    title: "Customer Success",
    description: "For CSMs managing customer accounts",
    icon: Building2,
    features: [
      "Account health tracking",
      "Customer meeting prep with ARR, lifecycle, health",
      "Stakeholder maps and relationship context",
      "Account-based action tracking",
    ],
  },
  {
    id: "general",
    title: "General",
    description: "For knowledge workers and project managers",
    icon: FolderKanban,
    features: [
      "Project-based organization",
      "Meeting prep with attendee context",
      "Action tracking by project",
      "External meeting research",
    ],
  },
];

interface ProfileSelectorProps {
  open: boolean;
  onProfileSet: (profile: ProfileType) => void;
}

export function ProfileSelector({ open, onProfileSet }: ProfileSelectorProps) {
  const [selected, setSelected] = useState<ProfileType | null>(null);
  const [saving, setSaving] = useState(false);

  async function handleSelect(profileId: ProfileType) {
    setSelected(profileId);
    setSaving(true);

    try {
      await invoke("set_profile", { profile: profileId });
      onProfileSet(profileId);
    } catch (err) {
      console.error("Failed to set profile:", err);
      setSelected(null);
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open}>
      <DialogContent
        className="sm:max-w-lg [&>button]:hidden"
        onPointerDownOutside={(e) => e.preventDefault()}
        onEscapeKeyDown={(e) => e.preventDefault()}
      >
        <DialogHeader>
          <DialogTitle>Welcome to DailyOS</DialogTitle>
          <DialogDescription>
            Choose your profile to customize how DailyOS prepares your day.
            You can change this later in Settings.
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-3 pt-2">
          {profiles.map((profile) => {
            const Icon = profile.icon;
            const isSelected = selected === profile.id;

            return (
              <Card
                key={profile.id}
                className={cn(
                  "cursor-pointer transition-all hover:-translate-y-0.5 hover:shadow-lg",
                  isSelected && "border-primary ring-1 ring-primary",
                  saving && !isSelected && "opacity-50 pointer-events-none"
                )}
                onClick={() => !saving && handleSelect(profile.id)}
              >
                <CardHeader className="pb-3">
                  <div className="flex items-start justify-between">
                    <div className="flex items-center gap-3">
                      <div className="flex size-10 items-center justify-center rounded-lg bg-muted">
                        <Icon className="size-5" />
                      </div>
                      <div>
                        <CardTitle className="text-base">{profile.title}</CardTitle>
                        <CardDescription>{profile.description}</CardDescription>
                      </div>
                    </div>
                    {isSelected && (
                      <div className="flex size-6 items-center justify-center rounded-full bg-primary text-primary-foreground">
                        <Check className="size-4" />
                      </div>
                    )}
                  </div>
                  <ul className="mt-3 space-y-1 text-sm text-muted-foreground">
                    {profile.features.map((feature) => (
                      <li key={feature} className="flex items-center gap-2">
                        <span className="size-1 rounded-full bg-muted-foreground/40" />
                        {feature}
                      </li>
                    ))}
                  </ul>
                </CardHeader>
              </Card>
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
}

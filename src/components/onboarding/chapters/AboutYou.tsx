import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { User, Building2, Briefcase, Globe, Target, ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";

interface AboutYouProps {
  onNext: () => void;
}

export function AboutYou({ onNext }: AboutYouProps) {
  const { email } = useGoogleAuth();

  const [name, setName] = useState("");
  const [company, setCompany] = useState("");
  const [title, setTitle] = useState("");
  const [domain, setDomain] = useState("");
  const [focus, setFocus] = useState("");

  // Pre-fill domain from Google email
  useEffect(() => {
    if (email) {
      const at = email.indexOf("@");
      if (at !== -1) {
        setDomain(email.slice(at + 1));
      }
    }
  }, [email]);

  async function handleContinue() {
    try {
      await invoke("set_user_profile", {
        name: name.trim() || null,
        company: company.trim() || null,
        title: title.trim() || null,
        focus: focus.trim() || null,
        domain: domain.trim() || null,
      });
    } catch (e) {
      console.error("set_user_profile failed:", e);
    }
    onNext();
  }

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-2xl font-semibold tracking-tight">About you</h2>
        <p className="text-sm text-muted-foreground">
          A little context helps DailyOS tailor your briefings. Everything here is optional.
        </p>
      </div>

      <div className="space-y-4">
        {/* Name */}
        <div className="space-y-1.5">
          <label className="text-sm font-medium flex items-center gap-2">
            <User className="size-4 text-muted-foreground" />
            Your name
          </label>
          <Input
            type="text"
            placeholder="e.g. Jamie Giroux"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>

        {/* Company */}
        <div className="space-y-1.5">
          <label className="text-sm font-medium flex items-center gap-2">
            <Building2 className="size-4 text-muted-foreground" />
            Company
          </label>
          <Input
            type="text"
            placeholder="e.g. Acme Inc."
            value={company}
            onChange={(e) => setCompany(e.target.value)}
          />
        </div>

        {/* Title */}
        <div className="space-y-1.5">
          <label className="text-sm font-medium flex items-center gap-2">
            <Briefcase className="size-4 text-muted-foreground" />
            Title
          </label>
          <Input
            type="text"
            placeholder="e.g. Customer Success Manager"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
        </div>

        {/* Domain */}
        <div className="space-y-1.5">
          <label className="text-sm font-medium flex items-center gap-2">
            <Globe className="size-4 text-muted-foreground" />
            Company domain
          </label>
          <Input
            type="text"
            placeholder="e.g. mycompany.com"
            value={domain}
            onChange={(e) => setDomain(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            Helps DailyOS tell your internal meetings apart from external ones.
          </p>
        </div>

        {/* Focus / priorities */}
        <div className="space-y-1.5">
          <label className="text-sm font-medium flex items-center gap-2">
            <Target className="size-4 text-muted-foreground" />
            Current priorities
          </label>
          <textarea
            className="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
            placeholder="e.g. Driving renewals for Q2, onboarding three new accounts"
            value={focus}
            onChange={(e) => setFocus(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            Share what you're focused on. This helps AI tailor your briefings.
          </p>
        </div>
      </div>

      <div className="flex justify-end">
        <Button onClick={handleContinue}>
          Continue
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}

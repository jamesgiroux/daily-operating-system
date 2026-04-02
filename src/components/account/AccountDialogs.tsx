import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { AccountMergeDialog } from "@/components/account/AccountMergeDialog";

import shared from "@/styles/entity-detail.module.css";

interface AccountDialogsProps {
  accountId: string;
  accountName: string;
  accountType: string;

  archiveDialogOpen: boolean;
  onArchiveDialogChange: (open: boolean) => void;
  onArchive: () => void;

  createChildOpen: boolean;
  onCreateChildOpenChange: (open: boolean) => void;
  childName: string;
  onChildNameChange: (value: string) => void;
  childDescription: string;
  onChildDescriptionChange: (value: string) => void;
  creatingChild: boolean;
  onCreateChild: () => void;

  mergeDialogOpen: boolean;
  onMergeDialogChange: (open: boolean) => void;
  onMerged: () => void;
}

export function AccountDialogs({
  accountId,
  accountName,
  accountType,
  archiveDialogOpen,
  onArchiveDialogChange,
  onArchive,
  createChildOpen,
  onCreateChildOpenChange,
  childName,
  onChildNameChange,
  childDescription,
  onChildDescriptionChange,
  creatingChild,
  onCreateChild,
  mergeDialogOpen,
  onMergeDialogChange,
  onMerged,
}: AccountDialogsProps) {
  return (
    <>
      {/* Archive Confirmation */}
      <AlertDialog open={archiveDialogOpen} onOpenChange={onArchiveDialogChange}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Archive Account</AlertDialogTitle>
            <AlertDialogDescription>
              This will hide {accountName} from active views.
              You can unarchive it later.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={onArchive}>Archive</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Child Account Creation */}
      <Dialog open={createChildOpen} onOpenChange={onCreateChildOpenChange}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {accountType === "internal" ? "Create Team" : "Create Business Unit"}
            </DialogTitle>
            <DialogDescription>
              Create a new {accountType === "internal" ? "team" : "business unit"} under {accountName}.
            </DialogDescription>
          </DialogHeader>
          <div className={shared.dialogForm}>
            <Input
              value={childName}
              onChange={(e) => onChildNameChange(e.target.value)}
              placeholder="Name"
            />
            <Input
              value={childDescription}
              onChange={(e) => onChildDescriptionChange(e.target.value)}
              placeholder="Description (optional)"
            />
            <div className={shared.dialogActions}>
              <Button
                variant="ghost"
                onClick={() => onCreateChildOpenChange(false)}
                className={shared.dialogButton}
              >
                Cancel
              </Button>
              <Button
                onClick={onCreateChild}
                disabled={creatingChild || !childName.trim()}
                className={shared.dialogButton}
              >
                {creatingChild ? "Creating\u2026" : "Create"}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <AccountMergeDialog
        open={mergeDialogOpen}
        onOpenChange={onMergeDialogChange}
        sourceAccountId={accountId}
        sourceAccountName={accountName}
        onMerged={onMerged}
      />
    </>
  );
}

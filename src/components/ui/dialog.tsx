import * as React from "react"
import * as DialogPrimitive from "@radix-ui/react-dialog"
import { X } from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "./button"

const Dialog = DialogPrimitive.Root

const DialogTrigger = DialogPrimitive.Trigger

const DialogPortal = DialogPrimitive.Portal

const DialogClose = DialogPrimitive.Close

const DialogOverlay = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Overlay>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Overlay>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Overlay
    ref={ref}
    className={cn(
      "fixed inset-0 z-50 bg-background/80 backdrop-blur-sm data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
      className
    )}
    {...props}
  />
))
DialogOverlay.displayName = DialogPrimitive.Overlay.displayName

const DialogContent = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Content> & { hideClose?: boolean }
>(({ className, children, hideClose, ...props }, ref) => (
  <DialogPortal>
    <DialogOverlay />
    <DialogPrimitive.Content
      ref={ref}
      className={cn(
        "fixed left-[50%] top-[50%] z-50 grid w-full max-w-md translate-x-[-50%] translate-y-[-50%] gap-0 border border-border/60 bg-card p-0 shadow-2xl duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] rounded-xl overflow-hidden",
        className
      )}
      {...props}
    >
      {children}
      {!hideClose && (
        <DialogPrimitive.Close className="absolute right-4 top-4 rounded-md p-1 text-muted-foreground/60 hover:bg-secondary hover:text-foreground transition-colors outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2">
          <X size={15} />
          <span className="sr-only">Close</span>
        </DialogPrimitive.Close>
      )}
    </DialogPrimitive.Content>
  </DialogPortal>
))
DialogContent.displayName = DialogPrimitive.Content.displayName


const DialogHeader = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn(
      "flex items-center justify-between border-b border-border/40 px-5 py-3.5",
      className
    )}
    {...props}
  />
)
DialogHeader.displayName = "DialogHeader"

const DialogFooter = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn(
      "flex items-center justify-end gap-2 border-t border-border/40 bg-secondary/20 px-5 py-3.5",
      className
    )}
    {...props}
  />
)
DialogFooter.displayName = "DialogFooter"

const DialogTitle = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Title>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Title>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Title
    ref={ref}
    className={cn(
      "text-[13.5px] font-semibold tracking-tight",
      className
    )}
    {...props}
  />
))
DialogTitle.displayName = DialogPrimitive.Title.displayName

const DialogDescription = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Description>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Description>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Description
    ref={ref}
    className={cn("text-[12.5px] text-muted-foreground/80 leading-relaxed", className)}
    {...props}
  />
))
DialogDescription.displayName = DialogPrimitive.Description.displayName

/* --- Legacy Compatibility Wrappers (for easier migration) --- */

interface LegacyDialogProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children?: React.ReactNode;
  footer?: React.ReactNode;
  className?: string;
}

export function LegacyDialog({
  isOpen,
  onClose,
  title,
  description,
  children,
  footer,
  className,
}: LegacyDialogProps) {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className={className}>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <div className="px-5 py-4">
          {description && <DialogDescription className="mb-4">{description}</DialogDescription>}
          {children}
        </div>
        {footer && <DialogFooter>{footer}</DialogFooter>}
      </DialogContent>
    </Dialog>
  );
}

interface ConfirmDialogProps extends Omit<LegacyDialogProps, "children" | "footer"> {
  onConfirm: () => void;
  confirmText?: string;
  cancelText?: string;
  variant?: "default" | "destructive";
}

export function ConfirmDialog({
  onConfirm,
  confirmText = "Confirm",
  cancelText = "Cancel",
  variant = "default",
  onClose,
  ...props
}: ConfirmDialogProps) {
  return (
    <LegacyDialog
      {...props}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={onClose} className="text-[12px]">
            {cancelText}
          </Button>
          <Button
            variant={variant === "destructive" ? "destructive" : "default"}
            size="sm"
            onClick={() => {
              onConfirm();
              onClose();
            }}
            className="text-[12px] px-4"
          >
            {confirmText}
          </Button>
        </>
      }
    />
  );
}

interface PromptDialogProps extends Omit<LegacyDialogProps, "children" | "footer"> {
  onConfirm: (value: string) => void;
  placeholder?: string;
  defaultValue?: string;
  confirmText?: string;
  cancelText?: string;
}

export function PromptDialog({
  onConfirm,
  placeholder,
  defaultValue = "",
  confirmText = "Create",
  cancelText = "Cancel",
  onClose,
  isOpen,
  ...props
}: PromptDialogProps) {
  const [value, setValue] = React.useState(defaultValue);

  React.useEffect(() => {
    if (isOpen) {
      setValue(defaultValue);
    }
  }, [defaultValue, isOpen]);

  const handleSubmit = (e?: React.FormEvent) => {
    e?.preventDefault();
    if (value.trim()) {
      onConfirm(value.trim());
      onClose();
    }
  };

  return (
    <LegacyDialog
      {...props}
      isOpen={isOpen}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={onClose} className="text-[12px]">
            {cancelText}
          </Button>
          <Button
            size="sm"
            disabled={!value.trim()}
            onClick={() => handleSubmit()}
            className="text-[12px] px-4"
          >
            {confirmText}
          </Button>
        </>
      }
    >
      <form onSubmit={handleSubmit}>
        <input
          autoFocus
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={placeholder}
          className="w-full bg-secondary/50 border border-border/60 rounded-md px-3 py-2 text-[13px] outline-none focus:border-blue-500/50 focus:ring-2 focus:ring-blue-500/10 transition-all"
        />
      </form>
    </LegacyDialog>
  );
}

export {
  Dialog,
  DialogPortal,
  DialogOverlay,
  DialogClose,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogFooter,
  DialogTitle,
  DialogDescription,
  LegacyDialog as CustomDialog, // Renamed for export
}

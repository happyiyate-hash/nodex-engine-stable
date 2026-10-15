import { Link } from "@tanstack/react-router";
import { ChevronLeft } from "lucide-react";
import { Button } from "@/components/ui/button";

export function PageHeader({
  title,
  backTo = "/",
}: {
  title: string;
  backTo?: string;
}) {
  return (
    <header className="mb-8 flex items-center gap-2">
      <Button variant="ghost" size="icon-sm" asChild aria-label="Back">
        <Link to={backTo}>
          <ChevronLeft className="size-5" />
        </Link>
      </Button>
      <h1 className="text-lg font-medium tracking-tight">{title}</h1>
    </header>
  );
}

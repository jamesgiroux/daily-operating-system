import { ScrollArea } from "@/components/ui/scroll-area";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

export function DashboardSkeleton() {
  return (
    <ScrollArea className="flex-1">
      <div className="px-8 pt-10 pb-8">
        <div className="mx-auto max-w-6xl">
          <div className="space-y-8">
            <div className="space-y-1">
              <Skeleton className="h-8 w-72" />
            </div>

            {/* Focus skeleton */}
            <div className="rounded-lg border p-3.5">
              <div className="flex items-start gap-2.5">
                <Skeleton className="size-4 shrink-0 mt-0.5" />
                <div className="min-w-0 space-y-1.5 flex-1">
                  <Skeleton className="h-3 w-10" />
                  <Skeleton className="h-4 w-full max-w-md" />
                </div>
              </div>
            </div>

            {/* Meeting card skeletons */}
            <div className="space-y-4">
              {[...Array(4)].map((_, i) => (
                <Card key={i}>
                  <CardContent className="p-5">
                    <div className="space-y-2">
                      <div className="flex items-center gap-2">
                        <Skeleton className="h-4 w-16" />
                        <Skeleton className="h-4 w-4" />
                        <Skeleton className="h-4 w-16" />
                      </div>
                      <Skeleton className="h-5 w-48" />
                      <Skeleton className="h-4 w-24" />
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>

            {/* Actions skeleton */}
            <section>
              <div className="flex items-center justify-between mb-3">
                <Skeleton className="h-3 w-20" />
                <Skeleton className="h-3 w-12" />
              </div>
              <div className="space-y-2">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className="flex items-center gap-3 rounded-md p-3">
                    <Skeleton className="size-5 rounded-full" />
                    <div className="flex-1 space-y-1">
                      <Skeleton className="h-4 w-full max-w-sm" />
                      <Skeleton className="h-3 w-20" />
                    </div>
                  </div>
                ))}
              </div>
            </section>
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}

import { ScrollArea } from "@/components/ui/scroll-area";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

export function DashboardSkeleton() {
  return (
    <ScrollArea className="flex-1">
      <div className="p-8">
        <div className="mx-auto max-w-6xl">
          <div className="grid gap-8 lg:grid-cols-[5fr_2fr]">
            {/* Left: Schedule skeleton */}
            <div className="min-w-0 space-y-6">
              <div className="space-y-1">
                <Skeleton className="h-8 w-72" />
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
            </div>

            {/* Right: Context sidebar skeleton */}
            <div className="min-w-0 space-y-5">
              {/* Focus skeleton */}
              <div className="rounded-lg border p-3.5">
                <div className="flex items-start gap-2.5">
                  <Skeleton className="size-4 shrink-0 mt-0.5" />
                  <div className="min-w-0 space-y-1.5 flex-1">
                    <Skeleton className="h-3 w-10" />
                    <Skeleton className="h-4 w-full" />
                    <Skeleton className="h-4 w-3/4" />
                  </div>
                </div>
              </div>

              {/* Actions sidebar skeleton */}
              <Card>
                <CardHeader>
                  <Skeleton className="h-6 w-24" />
                </CardHeader>
                <CardContent className="space-y-3">
                  {[...Array(4)].map((_, i) => (
                    <div key={i} className="flex items-center gap-3">
                      <Skeleton className="h-4 w-4" />
                      <div className="flex-1 space-y-1">
                        <Skeleton className="h-4 w-full" />
                        <Skeleton className="h-3 w-20" />
                      </div>
                    </div>
                  ))}
                </CardContent>
              </Card>
            </div>
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}

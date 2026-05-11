import { Navigate, useParams } from "@tanstack/react-router";

export default function MeetingHistoryDetailPage() {
  const { meetingId } = useParams({ strict: false });
  if (!meetingId) {
    return <Navigate to="/" replace />;
  }
  return <Navigate to="/meeting/$meetingId" params={{ meetingId }} replace />;
}

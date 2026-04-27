export default function Loading() {
  return (
    <div className="min-h-screen bg-gray-50 flex items-center justify-center">
      <div className="text-center space-y-4">
        <div className="skeleton h-12 w-12 rounded-full mx-auto" />
        <div className="space-y-2">
          <div className="skeleton h-4 w-32 mx-auto" />
          <div className="skeleton h-3 w-24 mx-auto" />
        </div>
      </div>
    </div>
  );
}

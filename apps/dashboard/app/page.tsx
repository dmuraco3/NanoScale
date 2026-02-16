import { AuthGuard } from "@/components/auth-guard";

async function HomePage() {
  return (
    <main className="min-h-screen bg-zinc-950 p-6 text-zinc-100">
      <h1 className="text-2xl font-semibold">NanoScale</h1>
      <p className="mt-2 text-zinc-300">Authenticated dashboard session active.</p>
    </main>
  );
}

export default AuthGuard(HomePage);

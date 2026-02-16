"use client";

import { useState } from "react";

import { clientApiBaseUrl } from "@/lib/auth-api";

export default function SetupForm() {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [errorMessage, setErrorMessage] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (password !== confirmPassword) {
      setErrorMessage("Passwords do not match.");
      return;
    }

    setIsSubmitting(true);
    setErrorMessage("");

    const response = await fetch(`${clientApiBaseUrl()}/api/auth/setup`, {
      method: "POST",
      credentials: "include",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({ username, password }),
    });

    if (response.ok) {
      window.location.assign("/");
      return;
    }

    if (response.status === 409) {
      setErrorMessage("Setup has already been completed. Please sign in.");
    } else {
      setErrorMessage("Unable to create admin account.");
    }

    setIsSubmitting(false);
  }

  return (
    <main className="min-h-screen flex items-center justify-center bg-zinc-950 px-4">
      <section className="w-full max-w-md rounded-lg border border-zinc-800 bg-zinc-900 p-6">
        <h1 className="text-xl font-semibold text-zinc-100">Welcome to NanoScale</h1>
        <form className="mt-6 space-y-4" onSubmit={handleSubmit}>
          <div>
            <label className="mb-1 block text-sm text-zinc-300" htmlFor="username">
              Username
            </label>
            <input
              id="username"
              className="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-zinc-100"
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              required
            />
          </div>

          <div>
            <label className="mb-1 block text-sm text-zinc-300" htmlFor="password">
              Password
            </label>
            <input
              id="password"
              type="password"
              className="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-zinc-100"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              required
            />
          </div>

          <div>
            <label className="mb-1 block text-sm text-zinc-300" htmlFor="confirm-password">
              Confirm Password
            </label>
            <input
              id="confirm-password"
              type="password"
              className="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-zinc-100"
              value={confirmPassword}
              onChange={(event) => setConfirmPassword(event.target.value)}
              required
            />
          </div>

          <button
            type="submit"
            disabled={isSubmitting}
            className="w-full rounded bg-zinc-100 px-3 py-2 text-zinc-900 disabled:opacity-70"
          >
            {isSubmitting ? "Creating..." : "Create Admin Account"}
          </button>
        </form>

        {errorMessage ? (
          <p className="mt-4 text-sm text-red-300" role="alert">
            {errorMessage}
          </p>
        ) : null}
      </section>
    </main>
  );
}

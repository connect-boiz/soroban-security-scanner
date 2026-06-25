"use client";

import React, { useState } from 'react';
import { MessageSquare, Send, CheckCircle2 } from 'lucide-react';

export default function Feedback() {
  const [submitted, setSubmitted] = useState(false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // Simulate form submission
    setTimeout(() => {
      setSubmitted(true);
    }, 600);
  };

  return (
    <div className="py-12 px-8 max-w-3xl mx-auto min-h-screen">
      <div className="mb-12">
        <h1 className="text-4xl font-extrabold text-white mb-4 flex items-center gap-3">
          <MessageSquare className="h-8 w-8 text-cyan-400" />
          Developer Feedback
        </h1>
        <p className="text-xl text-slate-400">
          Your feedback helps us improve the Soroban Security Scanner APIs and documentation.
        </p>
      </div>

      <div className="bg-[#151e32] border border-slate-700 rounded-2xl p-8 shadow-xl">
        {submitted ? (
          <div className="text-center py-16">
            <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-emerald-500/20 text-emerald-400 mb-6">
              <CheckCircle2 className="h-8 w-8" />
            </div>
            <h2 className="text-2xl font-bold text-white mb-2">Thank you!</h2>
            <p className="text-slate-400 mb-8">
              We've received your feedback and will review it shortly.
            </p>
            <button
              onClick={() => setSubmitted(false)}
              className="text-cyan-400 hover:text-cyan-300 font-semibold transition-colors"
            >
              Submit another response
            </button>
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="space-y-6">
            <div>
              <label htmlFor="topic" className="block text-sm font-medium text-slate-300 mb-2">
                What is this regarding?
              </label>
              <select
                id="topic"
                className="w-full bg-[#0d1117] border border-slate-700 rounded-lg px-4 py-3 text-slate-300 focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:border-transparent transition-all"
              >
                <option>API Documentation is unclear</option>
                <option>Found a bug in the SDK</option>
                <option>Feature request for the scanner</option>
                <option>Other / General Feedback</option>
              </select>
            </div>

            <div>
              <label htmlFor="message" className="block text-sm font-medium text-slate-300 mb-2">
                Detailed Feedback
              </label>
              <textarea
                id="message"
                rows={5}
                required
                placeholder="Please be as specific as possible..."
                className="w-full bg-[#0d1117] border border-slate-700 rounded-lg px-4 py-3 text-slate-300 focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:border-transparent transition-all resize-none"
              />
            </div>

            <div className="pt-4 border-t border-slate-800">
              <button
                type="submit"
                className="group relative flex w-full justify-center rounded-md bg-cyan-600 px-3 py-3 text-sm font-semibold text-white hover:bg-cyan-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-600 transition-colors"
              >
                <span className="absolute inset-y-0 left-0 flex items-center pl-3">
                  <Send className="h-5 w-5 text-cyan-200 group-hover:text-white transition-colors" aria-hidden="true" />
                </span>
                Send Feedback
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}

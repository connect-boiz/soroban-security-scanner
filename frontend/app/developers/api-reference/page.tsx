"use client";

import dynamic from 'next/dynamic';
import React, { useEffect, useState } from 'react';
import 'swagger-ui-react/swagger-ui.css';

// Dynamically import SwaggerUI to prevent SSR issues
const SwaggerUI = dynamic(() => import('swagger-ui-react'), { ssr: false });

export default function ApiReference() {
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  return (
    <div className="py-12 px-4 sm:px-6 lg:px-8 max-w-[90rem] mx-auto min-h-screen">
      <div className="mb-8">
        <h1 className="text-4xl font-extrabold tracking-tight text-white mb-4">
          Interactive API Reference
        </h1>
        <p className="text-lg text-slate-400">
          Explore our endpoints, examine request/response schemas, and even test them live 
          (requires a valid Bearer token).
        </p>
      </div>

      <div className="bg-white rounded-2xl shadow-xl overflow-hidden min-h-[600px] border border-slate-200">
        {mounted ? (
          <div className="swagger-container custom-swagger-styling">
            <style jsx global>{`
              /* Custom overrides for swagger-ui to make it look a bit better */
              .swagger-ui .info {
                margin: 20px 0;
              }
              .swagger-ui .wrapper {
                max-width: 100%;
                padding: 0 20px;
              }
              .swagger-ui .opblock-tag {
                font-family: inherit;
              }
            `}</style>
            <SwaggerUI url="/openapi.yaml" docExpansion="list" />
          </div>
        ) : (
          <div className="flex items-center justify-center h-96">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-cyan-500"></div>
          </div>
        )}
      </div>
    </div>
  );
}

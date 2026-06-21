import React, { useEffect, useRef } from 'react';
import { HELP_CONTENT, HelpTopic } from '../../lib/help-content';

interface HelpPanelProps {
  topic: HelpTopic | null;
  onClose: () => void;
}

/**
 * HelpPanel component - A slide-in drawer from the right side of the screen.
 * Shows detailed documentation for a specific topic.
 */
const HelpPanel: React.FC<HelpPanelProps> = ({ topic, onClose }) => {
  const panelRef = useRef<HTMLDivElement>(null);

  // Handle Escape key to close the panel
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };

    if (topic) {
      document.addEventListener('keydown', handleEscape);
      // Focus the close button or first element when opened
      setTimeout(() => {
        const closeBtn = panelRef.current?.querySelector('button');
        if (closeBtn) (closeBtn as HTMLButtonElement).focus();
      }, 100);
    }

    return () => {
      document.removeEventListener('keydown', handleEscape);
    };
  }, [topic, onClose]);

  if (!topic) return null;

  const content = HELP_CONTENT[topic];

  return (
    <div
      className="fixed inset-0 z-[100] flex justify-end bg-black/30 backdrop-blur-sm transition-opacity animate-fade-in"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-labelledby="help-panel-title"
    >
      <div
        ref={panelRef}
        className="w-full max-w-md h-full bg-white shadow-2xl p-0 overflow-y-auto transform transition-transform duration-300 ease-out translate-x-0"
        style={{ animation: 'slideInRight 0.3s ease-out' }}
        onClick={e => e.stopPropagation()}
      >
        <div className="sticky top-0 bg-white/80 backdrop-blur-md border-b px-6 py-4 flex items-center justify-between z-10">
          <h2 id="help-panel-title" className="text-xl font-bold text-gray-900">
            {content.title}
          </h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-100 rounded-full transition-colors text-gray-500 hover:text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500"
            aria-label="Close help panel"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-6 w-6"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        <div className="p-6 space-y-8">
          <div className="space-y-4">
            <p className="text-gray-600 text-lg leading-relaxed">{content.description}</p>
          </div>

          <div className="space-y-6">
            <h3 className="text-xs font-bold text-gray-400 uppercase tracking-widest">
              In-Depth Concepts
            </h3>
            <div className="grid grid-cols-1 gap-4">
              {Object.entries(content).map(([key, value]) => {
                if (key === 'title' || key === 'description') return null;
                return (
                  <div
                    key={key}
                    className="bg-gray-50 p-5 rounded-2xl border border-gray-100 hover:border-blue-200 hover:bg-blue-50/30 transition-all duration-200"
                  >
                    <h4 className="text-sm font-bold text-gray-900 capitalize mb-2 flex items-center">
                      <span className="w-1.5 h-1.5 bg-blue-500 rounded-full mr-2"></span>
                      {key.replace(/([A-Z])/g, ' $1').trim()}
                    </h4>
                    <p className="text-sm text-gray-600 leading-relaxed">{value}</p>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="pt-8 mt-4 border-t">
            <a
              href="#"
              className="group flex items-center justify-between p-4 bg-blue-600 rounded-2xl text-white font-bold hover:bg-blue-700 transition-all shadow-lg shadow-blue-200"
            >
              <span>Learn more in our Docs</span>
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-5 w-5 transform group-hover:translate-x-1 transition-transform"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M14 5l7 7m0 0l-7 7m7-7H3"
                />
              </svg>
            </a>
          </div>
        </div>
      </div>

      <style jsx global>{`
        @keyframes slideInRight {
          from {
            transform: translateX(100%);
          }
          to {
            transform: translateX(0);
          }
        }
      `}</style>
    </div>
  );
};

export default HelpPanel;

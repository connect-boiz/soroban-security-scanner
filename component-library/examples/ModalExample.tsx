'use client';

import React, { useState } from 'react';
import Modal from '../src/components/Modal';
import Dialog from '../src/components/Dialog';

const ModalExample: React.FC = () => {
  const [isBasicModalOpen, setIsBasicModalOpen] = useState(false);
  const [isLargeModalOpen, setIsLargeModalOpen] = useState(false);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [isDangerDialogOpen, setIsDangerDialogOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const handleConfirm = async () => {
    setIsLoading(true);
    // Simulate async operation
    await new Promise(resolve => setTimeout(resolve, 2000));
    setIsLoading(false);
    console.log('Action confirmed!');
  };

  return (
    <div className="p-8 space-y-8">
      <h1 className="text-3xl font-bold text-gray-900 mb-8">Modal & Dialog Examples</h1>

      {/* Basic Modal Examples */}
      <div className="space-y-4">
        <h2 className="text-2xl font-semibold text-gray-800">Modal Examples</h2>
        
        <div className="flex flex-wrap gap-4">
          <button
            onClick={() => setIsBasicModalOpen(true)}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
          >
            Open Basic Modal
          </button>

          <button
            onClick={() => setIsLargeModalOpen(true)}
            className="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors"
          >
            Open Large Modal
          </button>
        </div>
      </div>

      {/* Dialog Examples */}
      <div className="space-y-4">
        <h2 className="text-2xl font-semibold text-gray-800">Dialog Examples</h2>
        
        <div className="flex flex-wrap gap-4">
          <button
            onClick={() => setIsDialogOpen(true)}
            className="px-4 py-2 bg-purple-600 text-white rounded-md hover:bg-purple-700 transition-colors"
          >
            Open Confirmation Dialog
          </button>

          <button
            onClick={() => setIsDangerDialogOpen(true)}
            className="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 transition-colors"
          >
            Open Danger Dialog
          </button>
        </div>
      </div>

      {/* Basic Modal */}
      <Modal
        isOpen={isBasicModalOpen}
        onClose={() => setIsBasicModalOpen(false)}
        title="Basic Modal"
        size="md"
      >
        <div className="space-y-4">
          <p>This is a basic modal example with standard functionality.</p>
          <p>It includes:</p>
          <ul className="list-disc list-inside space-y-1 text-gray-700">
            <li>Focus management</li>
            <li>Keyboard navigation</li>
            <li>Backdrop click to close</li>
            <li>Escape key support</li>
            <li>Responsive design</li>
          </ul>
          <button
            onClick={() => setIsBasicModalOpen(false)}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
          >
            Close Modal
          </button>
        </div>
      </Modal>

      {/* Large Modal */}
      <Modal
        isOpen={isLargeModalOpen}
        onClose={() => setIsLargeModalOpen(false)}
        title="Large Modal"
        size="lg"
      >
        <div className="space-y-6">
          <p>This is a larger modal with more content to demonstrate scrolling behavior.</p>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="bg-gray-50 p-4 rounded-lg">
              <h3 className="font-semibold mb-2">Feature 1</h3>
              <p className="text-gray-600">Description of the first feature with details about its functionality.</p>
            </div>
            <div className="bg-gray-50 p-4 rounded-lg">
              <h3 className="font-semibold mb-2">Feature 2</h3>
              <p className="text-gray-600">Description of the second feature with details about its functionality.</p>
            </div>
            <div className="bg-gray-50 p-4 rounded-lg">
              <h3 className="font-semibold mb-2">Feature 3</h3>
              <p className="text-gray-600">Description of the third feature with details about its functionality.</p>
            </div>
            <div className="bg-gray-50 p-4 rounded-lg">
              <h3 className="font-semibold mb-2">Feature 4</h3>
              <p className="text-gray-600">Description of the fourth feature with details about its functionality.</p>
            </div>
          </div>

          <div className="pt-4 border-t border-gray-200">
            <button
              onClick={() => setIsLargeModalOpen(false)}
              className="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors"
            >
              Close Large Modal
            </button>
          </div>
        </div>
      </Modal>

      {/* Confirmation Dialog */}
      <Dialog
        isOpen={isDialogOpen}
        onClose={() => setIsDialogOpen(false)}
        title="Confirm Action"
        message="Are you sure you want to proceed with this action? This will update your settings."
        onConfirm={handleConfirm}
        isConfirmLoading={isLoading}
        confirmText="Confirm"
        cancelText="Cancel"
        variant="default"
      />

      {/* Danger Dialog */}
      <Dialog
        isOpen={isDangerDialogOpen}
        onClose={() => setIsDangerDialogOpen(false)}
        title="Delete Item"
        message="Are you sure you want to delete this item? This action cannot be undone."
        onConfirm={() => {
          console.log('Item deleted!');
          setIsDangerDialogOpen(false);
        }}
        confirmText="Delete"
        cancelText="Cancel"
        variant="danger"
      />
    </div>
  );
};

export default ModalExample;

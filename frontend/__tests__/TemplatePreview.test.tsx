import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { TemplatePreview } from '../components/notifications/TemplatePreview';

const template = {
  id: 't1',
  name: 'Vulnerability Alert',
  description: 'Alert for new findings',
  supported_channels: ['email', 'in_app'],
  variables: [
    { name: 'user_name', required: true, variable_type: 'String' as const },
    { name: 'severity', required: false, variable_type: 'String' as const, default_value: 'High' },
    { name: 'count', required: false, variable_type: 'Number' as const },
  ],
  version: 2,
  active: true,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-02T00:00:00Z',
};

const renderedPreview = {
  subject: 'Alert for {{user_name}}',
  plain_text_body: 'Hello {{user_name}}, severity is {{severity}}',
  html_body: '<b>Hello {{user_name}}</b>',
  template_id: 't1',
  template_name: 'Vulnerability Alert',
};

const renderPreview = jest.fn();

describe('TemplatePreview', () => {
  beforeEach(() => {
    renderPreview.mockReset();
  });

  it('renders the header and empty state', () => {
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    expect(screen.getByText('Template Preview')).toBeInTheDocument();
    expect(screen.getByText('Choose a template...')).toBeInTheDocument();
    expect(screen.getByText('No Template Selected')).toBeInTheDocument();
  });

  it('lists templates in the selector', () => {
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    const select = screen.getByLabelText('Select Template');
    expect(select).toHaveTextContent('Vulnerability Alert (v2)');
  });

  it('shows template metadata after selection', () => {
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    fireEvent.change(screen.getByLabelText('Select Template'), { target: { value: 't1' } });
    expect(screen.getByText('Version 2')).toBeInTheDocument();
    expect(screen.getByText('2 channel(s)')).toBeInTheDocument();
    expect(screen.getByText('Active')).toBeInTheDocument();
    expect(screen.getByText('Alert for new findings')).toBeInTheDocument();
  });

  it('populates default values into the context editor', () => {
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    fireEvent.change(screen.getByLabelText('Select Template'), { target: { value: 't1' } });
    const textarea = screen.getByPlaceholderText(/"user_name"/) as HTMLTextAreaElement;
    expect(textarea.value).toContain('"severity": "High"');
  });

  it('flags missing required variables and disables the preview button', () => {
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    fireEvent.change(screen.getByLabelText('Select Template'), { target: { value: 't1' } });
    expect(screen.getByText('user_name:')).toBeInTheDocument();
    expect(screen.getByText('Required variable is missing')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /render preview/i })).toBeDisabled();
  });

  it('fills missing required variables with defaults', () => {
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    fireEvent.change(screen.getByLabelText('Select Template'), { target: { value: 't1' } });
    fireEvent.click(screen.getByRole('button', { name: /fill defaults/i }));
    const textarea = screen.getByPlaceholderText(/"user_name"/) as HTMLTextAreaElement;
    expect(textarea.value).toContain('"user_name": "{{user_name}}"');
    expect(screen.queryByText('Required variable is missing')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /render preview/i })).not.toBeDisabled();
  });

  it('clears the context', () => {
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    fireEvent.change(screen.getByLabelText('Select Template'), { target: { value: 't1' } });
    fireEvent.click(screen.getByRole('button', { name: /clear/i }));
    const textarea = screen.getByPlaceholderText(/"user_name"/) as HTMLTextAreaElement;
    expect(textarea.value).toBe('{}');
  });

  it('rejects invalid JSON context with a parse error', () => {
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    fireEvent.change(screen.getByLabelText('Select Template'), { target: { value: 't1' } });
    const textarea = screen.getByPlaceholderText(/"user_name"/) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: '{not json' } });
    expect(screen.getByText(/Expected property name|Unexpected token/)).toBeInTheDocument();
  });

  it('renders a preview with subject, body and HTML tab', async () => {
    renderPreview.mockResolvedValue(renderedPreview);
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    fireEvent.change(screen.getByLabelText('Select Template'), { target: { value: 't1' } });
    fireEvent.click(screen.getByRole('button', { name: /fill defaults/i }));
    fireEvent.click(screen.getByRole('button', { name: /render preview/i }));

    expect(await screen.findByText('Subject Line')).toBeInTheDocument();
    expect(screen.getByTitle('Email template preview')).toBeInTheDocument();
    expect(screen.getAllByText('{{user_name}}').length).toBeGreaterThan(0);
    expect(renderPreview).toHaveBeenCalledWith('t1', expect.objectContaining({ severity: 'High' }));
  });

  it('shows the plain text tab when no HTML body exists', async () => {
    renderPreview.mockResolvedValue({ ...renderedPreview, html_body: undefined });
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    fireEvent.change(screen.getByLabelText('Select Template'), { target: { value: 't1' } });
    fireEvent.click(screen.getByRole('button', { name: /fill defaults/i }));
    fireEvent.click(screen.getByRole('button', { name: /render preview/i }));

    expect(await screen.findByText('Plain Text')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /html/i })).not.toBeInTheDocument();
    expect(screen.getByText(/Hello/)).toBeInTheDocument();
  });

  it('displays an error when preview generation fails', async () => {
    renderPreview.mockRejectedValue(new Error('Rendering backend unavailable'));
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    fireEvent.change(screen.getByLabelText('Select Template'), { target: { value: 't1' } });
    fireEvent.click(screen.getByRole('button', { name: /fill defaults/i }));
    fireEvent.click(screen.getByRole('button', { name: /render preview/i }));

    expect(await screen.findByText('Rendering backend unavailable')).toBeInTheDocument();
  });

  it('copies the template config to the clipboard', async () => {
    const writeText = jest.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText },
      configurable: true,
    });
    render(<TemplatePreview templates={[template]} onPreview={renderPreview} />);
    fireEvent.change(screen.getByLabelText('Select Template'), { target: { value: 't1' } });
    await act(async () => {
      fireEvent.click(screen.getByTitle('Copy template config as JSON'));
    });
    expect(writeText).toHaveBeenCalledWith(expect.stringContaining('"template_id": "t1"'));
    expect(screen.getByText('Copied')).toBeInTheDocument();
  });

  it('refreshes templates via the fetch callback', () => {
    const onFetchTemplates = jest.fn().mockResolvedValue([template]);
    render(
      <TemplatePreview
        templates={[template]}
        onPreview={renderPreview}
        onFetchTemplates={onFetchTemplates}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /refresh templates/i }));
    expect(onFetchTemplates).toHaveBeenCalled();
  });

  it('shows refreshing state while templates are loading', () => {
    render(
      <TemplatePreview
        templates={[template]}
        onPreview={renderPreview}
        onFetchTemplates={jest.fn()}
        isLoading
      />
    );
    expect(screen.getByRole('button', { name: /refreshing/i })).toBeDisabled();
  });
});

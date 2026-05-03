import React, {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import clsx from 'clsx';
import {translate} from '@docusaurus/Translate';
import {useCodeBlockContext} from '@docusaurus/theme-common/internal';
import Button from '@theme/CodeBlock/Buttons/Button';
import type {Props} from '@theme/CodeBlock/Buttons/CopyButton';
import IconCopy from '@theme/Icon/Copy';
import IconSuccess from '@theme/Icon/Success';

import styles from './styles.module.css';

type CopyState = 'idle' | 'copied' | 'error';

function buttonTitle(copyState: CopyState) {
  if (copyState === 'copied') {
    return translate({
      id: 'theme.CodeBlock.copied',
      message: 'Copied',
      description: 'The copied button label on code blocks',
    });
  }

  if (copyState === 'error') {
    return translate({
      id: 'theme.CodeBlock.copyFailed',
      message: 'Copy unavailable',
      description: 'The copy button label after a clipboard failure',
    });
  }

  return translate({
    id: 'theme.CodeBlock.copy',
    message: 'Copy',
    description: 'The copy button label on code blocks',
  });
}

function ariaLabel(copyState: CopyState) {
  if (copyState === 'copied') {
    return translate({
      id: 'theme.CodeBlock.copied',
      message: 'Copied',
      description: 'The copied button label on code blocks',
    });
  }

  if (copyState === 'error') {
    return translate({
      id: 'theme.CodeBlock.copyFailedAriaLabel',
      message: 'Copy code unavailable in this browser',
      description: 'The ARIA label for copy code blocks button after a clipboard failure',
    });
  }

  return translate({
    id: 'theme.CodeBlock.copyButtonAriaLabel',
    message: 'Copy code to clipboard',
    description: 'The ARIA label for copy code blocks button',
  });
}

function fallbackCopyToClipboard(text: string): boolean {
  const textarea = document.createElement('textarea');
  const selection = document.getSelection();
  const previousRanges =
    selection == null
      ? []
      : Array.from({length: selection.rangeCount}, (_, index) =>
          selection.getRangeAt(index).cloneRange(),
        );

  textarea.value = text;
  textarea.setAttribute('readonly', '');
  textarea.style.left = '-9999px';
  textarea.style.opacity = '0';
  textarea.style.position = 'fixed';
  textarea.style.top = '0';

  document.body.appendChild(textarea);
  textarea.focus();
  textarea.select();
  textarea.setSelectionRange(0, textarea.value.length);

  try {
    return document.execCommand('copy');
  } finally {
    document.body.removeChild(textarea);
    if (selection != null) {
      selection.removeAllRanges();
      previousRanges.forEach((range) => selection.addRange(range));
    }
  }
}

async function copyToClipboard(text: string) {
  let clipboardError: unknown;

  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return;
    } catch (error) {
      clipboardError = error;
    }
  }

  if (fallbackCopyToClipboard(text)) {
    return;
  }

  throw clipboardError ?? new Error('Copy failed');
}

function useCopyButton() {
  const {
    metadata: {code},
  } = useCodeBlockContext();
  const [copyState, setCopyState] = useState<CopyState>('idle');
  const copyTimeout = useRef<number | undefined>(undefined);

  const clearCopyTimeout = useCallback(() => {
    window.clearTimeout(copyTimeout.current);
  }, []);

  const queueReset = useCallback(() => {
    clearCopyTimeout();
    copyTimeout.current = window.setTimeout(() => {
      setCopyState('idle');
    }, 1200);
  }, [clearCopyTimeout]);

  const copyCode = useCallback(async () => {
    try {
      await copyToClipboard(code);
      setCopyState('copied');
    } catch {
      setCopyState('error');
    } finally {
      queueReset();
    }
  }, [code, queueReset]);

  useEffect(() => () => clearCopyTimeout(), [clearCopyTimeout]);

  return {copyCode, copyState};
}

export default function CopyButton({className}: Props): ReactNode {
  const {copyCode, copyState} = useCopyButton();

  return (
    <Button
      aria-label={ariaLabel(copyState)}
      title={buttonTitle(copyState)}
      className={clsx(
        className,
        styles.copyButton,
        copyState === 'copied' && styles.copyButtonCopied,
        copyState === 'error' && styles.copyButtonError,
      )}
      onClick={() => {
        void copyCode();
      }}>
      <span className={styles.copyButtonIcons} aria-hidden="true">
        <IconCopy className={styles.copyButtonIcon} />
        <IconSuccess className={styles.copyButtonSuccessIcon} />
      </span>
    </Button>
  );
}

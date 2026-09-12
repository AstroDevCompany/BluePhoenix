import { useCallback, useEffect, useRef, useState } from "react";

export function useOpenTransition(durationMs = 180) {
  const [open, setOpen] = useState(false);
  const [visible, setVisible] = useState(false);
  const timer = useRef<number | null>(null);

  const clear = () => {
    if (timer.current != null) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
  };

  const show = useCallback(() => {
    clear();
    setVisible(true);
    setOpen(true);
  }, []);

  const hide = useCallback(() => {
    setOpen(false);
    clear();
    timer.current = window.setTimeout(() => {
      setVisible(false);
      timer.current = null;
    }, durationMs);
  }, [durationMs]);

  const toggle = useCallback(() => {
    if (open) hide();
    else show();
  }, [open, hide, show]);

  useEffect(() => () => clear(), []);

  return { open, visible, show, hide, toggle };
}

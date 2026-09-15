import {
  Children,
  isValidElement,
  memo,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import Markdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { Check, Copy } from "lucide-react";
import { api } from "../../lib/ipc";

function closeOpenFence(text: string) {
  const ticks = text.match(/```/g)?.length ?? 0;
  return ticks % 2 === 1 ? text + "\n```" : text;
}

function childText(node: ReactNode): string {
  if (node == null || typeof node === "boolean") return "";
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(childText).join("");
  if (isValidElement<{ children?: ReactNode }>(node)) return childText(node.props.children);
  return "";
}

function languageOf(children: ReactNode) {
  const child = Children.toArray(children)[0];
  if (!isValidElement<{ className?: string }>(child)) return;
  const match = /language-([a-z0-9_+-]+)/i.exec(child.props.className ?? "");
  return match?.[1];
}

function isSafeHref(href: string) {
  try {
    const url = new URL(href);
    return url.protocol === "http:" || url.protocol === "https:" || url.protocol === "mailto:";
  } catch {
    return false;
  }
}

function CodeBlock({ children }: { children?: ReactNode }) {
  const [copied, setCopied] = useState(false);
  const timer = useRef<number>(0);
  const lang = languageOf(children);

  useEffect(() => () => window.clearTimeout(timer.current), []);

  const copy = async () => {
    await navigator.clipboard.writeText(childText(children).replace(/\n$/, ""));
    setCopied(true);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setCopied(false), 1200);
  };

  return (
    <div className="ai-md-code">
      <div className="ai-md-code-bar">
        <span className="ai-md-lang">{lang ?? "code"}</span>
        <button type="button" className="ai-md-copy" onClick={() => void copy()}>
          {copied ? <Check size={12} /> : <Copy size={12} />}
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <pre>{children}</pre>
    </div>
  );
}

const components: Components = {
  a({ href, children }) {
    if (!href || !isSafeHref(href)) return <span>{children}</span>;
    return (
      <a
        href={href}
        onClick={(e) => {
          e.preventDefault();
          void api.openUrl(href);
        }}
      >
        {children}
      </a>
    );
  },
  img({ src, alt }) {
    if (src && /^(data:|asset:|blob:)/.test(src)) {
      return <img src={src} alt={alt ?? ""} />;
    }
    return alt ? <span className="ai-md-img-fallback">{alt}</span> : null;
  },
  pre({ children }) {
    return <CodeBlock>{children}</CodeBlock>;
  },
  table({ children }) {
    return (
      <div className="ai-md-table-wrap">
        <table>{children}</table>
      </div>
    );
  },
};

export const MarkdownMessage = memo(function MarkdownMessage({ text }: { text: string }) {
  if (!text) return null;
  return (
    <div className="ai-md">
      <Markdown remarkPlugins={[remarkGfm]} components={components}>
        {closeOpenFence(text)}
      </Markdown>
    </div>
  );
});

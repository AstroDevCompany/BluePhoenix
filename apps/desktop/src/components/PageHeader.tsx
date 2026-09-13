import type { ReactNode } from "react";

export function PageHeader({
  title,
  kicker,
  children,
}: {
  title: string;
  kicker?: string;
  children?: ReactNode;
}) {
  return (
    <header className="page-header">
      <div>
        {kicker ? <p className="page-kicker">{kicker}</p> : null}
        <h1 className="h1">{title}</h1>
      </div>
      {children}
    </header>
  );
}

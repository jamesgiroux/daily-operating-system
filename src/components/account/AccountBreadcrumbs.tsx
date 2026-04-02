import React from "react";
import { useNavigate } from "@tanstack/react-router";

import shared from "@/styles/entity-detail.module.css";
import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountBreadcrumbsProps {
  ancestors: { id: string; name: string }[];
  currentName: string;
}

export function AccountBreadcrumbs({ ancestors, currentName }: AccountBreadcrumbsProps) {
  const navigate = useNavigate();

  if (ancestors.length === 0) return null;

  return (
    <nav className={shared.breadcrumbNav}>
      <button
        onClick={() => navigate({ to: "/accounts" })}
        className={shared.breadcrumbButton}
      >
        Accounts
      </button>
      {ancestors.map((anc) => (
        <React.Fragment key={anc.id}>
          <span className={shared.breadcrumbSeparator}>/</span>
          <button
            onClick={() =>
              navigate({
                to: "/accounts/$accountId",
                params: { accountId: anc.id },
              })
            }
            className={styles.breadcrumbAncestorLink}
          >
            {anc.name}
          </button>
        </React.Fragment>
      ))}
      <span className={shared.breadcrumbSeparator}>/</span>
      <span className={shared.breadcrumbCurrent}>{currentName}</span>
    </nav>
  );
}

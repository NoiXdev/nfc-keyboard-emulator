export type NavView = "scanner" | "log" | "options" | "accessibility";

export interface NavItem {
  key: NavView;
  label: string;
}

export interface NavGroup {
  label: string;
  items: NavItem[];
}

export const NAV_GROUPS: NavGroup[] = [
  {
    label: "Scanner",
    items: [
      { key: "scanner", label: "Scanner" },
      { key: "log", label: "Log" },
    ],
  },
  {
    label: "Einstellungen",
    items: [
      { key: "options", label: "Optionen" },
      { key: "accessibility", label: "Bedienungshilfen" },
    ],
  },
];

export const VIEW_TITLES: Record<NavView, string> = {
  scanner: "Scanner",
  log: "Scan-Log",
  options: "Optionen",
  accessibility: "Bedienungshilfen",
};

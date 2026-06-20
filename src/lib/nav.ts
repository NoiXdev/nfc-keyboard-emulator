export type NavView = "scanner" | "log" | "options" | "accessibility" | "about";

export interface NavGroup {
  groupKey: "groupScanner" | "groupSettings";
  items: NavView[];
}

export const NAV_GROUPS: NavGroup[] = [
  { groupKey: "groupScanner", items: ["scanner", "log"] },
  { groupKey: "groupSettings", items: ["options", "accessibility"] },
];

import data from "../licenses.json";

export interface LicenseEntry {
  name: string;
  version: string;
  license: string;
}

const typed = data as { rust: LicenseEntry[]; npm: LicenseEntry[] };

export const rustLicenses = typed.rust;
export const npmLicenses = typed.npm;

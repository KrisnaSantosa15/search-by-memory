/** Why a file could not be read, as the index reports it. */
export type Unreadable =
  | "passwordProtected"
  | "tooLarge"
  | "damaged"
  | "inUse"
  | "noAccess"
  | "gone"
  | "other";

/** Each reason in words someone who never saw the error can act on. */
export const UNREADABLE: Record<Unreadable, string> = {
  passwordProtected: "Protected by a password",
  tooLarge: "Too large to read (more than 50 MB of text)",
  damaged: "The file seems damaged",
  inUse: "Another program had it open",
  noAccess: "Windows didn't allow reading it",
  gone: "Moved or deleted before it could be read",
  other: "Couldn't be read",
};

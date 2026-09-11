export interface Folder {
  id: string;
  kind: "folder";
  name: string;
  expanded: boolean;
  children: Node[];
}

export interface Workspace {
  id: string;
  kind: "workspace";
  name: string;
  path: string;
  aiToolId: string | null;
  shellId: string | null;
  rows: number;
  cols: number;
  rowSizes: number[];
  colSizes: number[];
}

export type Node = Folder | Workspace;

export interface AiTool {
  id: string;
  name: string;
  command: string;
}

export interface DirAlias {
  id: string;
  alias: string;
  path: string;
}

export interface Config {
  version: number;
  aiTools: AiTool[];
  directories: DirAlias[];
  recentDirs: string[];
  defaultShellId: string;
  tree: Node[];
}

export interface ShellInfo {
  id: string;
  name: string;
  program: string;
  args: string[];
}

export type LoadStatus =
  | { kind: "fresh" }
  | { kind: "loaded" }
  | { kind: "recovered"; backup: string };

export interface LoadResult {
  config: Config;
  status: LoadStatus;
}

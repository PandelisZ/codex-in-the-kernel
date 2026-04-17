import { execFileSync } from "node:child_process";

const run = (command, args, options = {}) =>
  execFileSync(command, args, {
    stdio: "inherit",
    ...options,
  });

const capture = (command, args) =>
  execFileSync(command, args, {
    encoding: "utf8",
  }).trim();

const fail = (message) => {
  console.error(`deploy: ${message}`);
  process.exit(1);
};

const ensureAvailable = (command, hint) => {
  try {
    capture("which", [command]);
  } catch {
    fail(hint);
  }
};

const parseRepoSlug = (remoteUrl) => {
  const match = remoteUrl.match(/github\.com[:/]([^/]+\/[^/.]+?)(?:\.git)?$/);

  if (!match) {
    fail(`could not infer GitHub repo slug from origin URL: ${remoteUrl}`);
  }

  return match[1];
};

ensureAvailable("gh", "GitHub CLI is required. Install `gh` and authenticate before running `pnpm run deploy`.");

const remoteUrl = capture("git", ["remote", "get-url", "origin"]);
const repoSlug = parseRepoSlug(remoteUrl);
const repoName = repoSlug.split("/")[1];
const currentBranch = capture("git", ["rev-parse", "--abbrev-ref", "HEAD"]);

if (currentBranch === "HEAD") {
  fail("you are in a detached HEAD state. Switch to a branch before deploying.");
}

const worktreeDirty = capture("git", ["status", "--short"]);
if (worktreeDirty.length > 0) {
  console.warn("deploy: warning: working tree is dirty.");
  console.warn("deploy: warning: GitHub Pages will deploy the pushed contents of the selected remote branch, not your uncommitted local changes.");
}

try {
  capture("git", ["ls-remote", "--exit-code", "--heads", "origin", currentBranch]);
} catch {
  fail(`branch \`${currentBranch}\` is not on origin. Push it first so GitHub Actions has a ref to deploy from.`);
}

const localHead = capture("git", ["rev-parse", "HEAD"]);
const remoteHead = capture("git", ["rev-parse", `origin/${currentBranch}`]);

if (localHead !== remoteHead) {
  console.warn(`deploy: warning: local HEAD (${localHead.slice(0, 7)}) does not match origin/${currentBranch} (${remoteHead.slice(0, 7)}).`);
  console.warn("deploy: warning: the workflow will deploy origin's commit, not your newer local commit.");
}

console.log(`deploy: building docs with GitHub Pages base /${repoName}/`);
run("pnpm", ["run", "docs:build"], {
  env: {
    ...process.env,
    PAGES_BASE: `/${repoName}/`,
    VITE_REPO_URL: `https://github.com/${repoSlug}`,
  },
});

if (process.env.DEPLOY_DRY_RUN === "1") {
  console.log("deploy: dry run enabled, skipping workflow dispatch.");
  process.exit(0);
}

console.log(`deploy: triggering GitHub Pages workflow for origin/${currentBranch}`);
run("gh", ["workflow", "run", "pages.yml", "--ref", currentBranch]);

console.log("deploy: workflow dispatched.");
console.log("deploy: check GitHub Actions for the Pages deployment status.");

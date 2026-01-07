#!/bin/bash
# git-remotes.sh - Manage remotes for yrmpc-ytmapi
# Usage: ./scripts/git-remotes.sh [setup|sync|status]

set -e

UPSTREAM_URL="https://github.com/nick42d/youtui.git"
UPSTREAM_BRANCH="master"
YTMAPI_PATH="ytmapi-rs"

case "${1:-status}" in
setup)
	echo "Setting up remotes..."

	# Add upstream if not exists
	if ! git remote get-url upstream &>/dev/null; then
		git remote add upstream "$UPSTREAM_URL"
		echo "Added upstream: $UPSTREAM_URL"
	else
		echo "Upstream already configured"
	fi

	git remote -v
	;;

sync)
	echo "Syncing from upstream nick42d/youtui..."

	# Ensure upstream exists
	if ! git remote get-url upstream &>/dev/null; then
		echo "Run './scripts/git-remotes.sh setup' first"
		exit 1
	fi

	# Fetch upstream
	git fetch upstream "$UPSTREAM_BRANCH"

	# Extract ytmapi-rs subtree from upstream
	echo "Extracting $YTMAPI_PATH from upstream/$UPSTREAM_BRANCH..."
	UPSTREAM_YTMAPI=$(git subtree split --prefix="$YTMAPI_PATH" upstream/$UPSTREAM_BRANCH)

	# Merge into current branch
	echo "Merging upstream changes..."
	git merge "$UPSTREAM_YTMAPI" -m "chore: sync from upstream nick42d/youtui"

	echo "Sync complete!"
	;;

status)
	echo "=== Remote Status ==="
	git remote -v
	echo ""
	echo "=== Current Branch ==="
	git branch -vv
	echo ""
	echo "=== Recent Commits ==="
	git log --oneline -5
	;;

*)
	echo "Usage: $0 [setup|sync|status]"
	echo ""
	echo "Commands:"
	echo "  setup   - Configure upstream remote (nick42d/youtui)"
	echo "  sync    - Pull latest ytmapi-rs changes from upstream"
	echo "  status  - Show current remote and branch status"
	exit 1
	;;
esac

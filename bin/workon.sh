#!/bin/bash

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]:-${(%):-%x}}" )" >/dev/null 2>&1 && pwd )"
PROJ_ROOT="${THIS_DIR}/.."
RUST_ROOT="${PROJ_ROOT}/invrs"

pushd ${PROJ_ROOT}>>/dev/null

# ----------------------------
# Environment vars
# ----------------------------

export PROJ_ROOT=${PROJ_ROOT}
export PS1="(sc2-exp) $PS1"

alias sc2-exp="cargo run -q --"

# ----------------------------
# SC2 deployment variables
# ----------------------------

export SC2_DEPLOY_SOURCE=~/git/sc2-sys/deploy
export KUBECONFIG=${SC2_DEPLOY_SOURCE}/.config/kubeadm_kubeconfig
alias k9s=${SC2_DEPLOY_SOURCE}/bin/k9s
alias kubectl=${SC2_DEPLOY_SOURCE}/bin/kubectl

# -----------------------------
# Splash
# -----------------------------

echo ""
echo "----------------------------------"
echo "SC2 Experiments CLI"
echo "----------------------------------"
echo ""

popd >> /dev/null

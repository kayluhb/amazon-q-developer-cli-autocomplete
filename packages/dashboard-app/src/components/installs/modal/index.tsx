import { InstallCheck } from "@/types/preferences";
import { Fig, Internal } from "@tab/api-bindings";
import { useEffect, useState } from "react";
import { Button } from "../../ui/button";
import onboarding from "@/data/onboarding";
import { useStatusCheck } from "@/hooks/store/useStatusCheck";
import InstallModal from "./install";
import { useRefreshLocalState } from "@/hooks/store/useState";
import appIcon from "@assets/images/icon.png?url";
import { usePlatformInfo } from "@/hooks/store/usePlatformInfo";
import { matchesPlatformRestrictions } from "@/lib/platform";

const INSTALL_STEP_IDS = ["dotfiles", "accessibility"];

export default function OnboardingModal() {
  const [step, setStep] = useState(0);
  const check = onboarding[step] as InstallCheck;
  const [dotfilesCheck, refreshDotfiles] = useStatusCheck("dotfiles");
  const [accessibilityCheck, refreshAccessibility] =
    useStatusCheck("accessibility");
  const refreshLocalState = useRefreshLocalState();
  const platformInfo = usePlatformInfo();

  const [_dotfiles, setDotfiles] = useState(dotfilesCheck);
  const [_accessibility, setAccessibility] = useState(accessibilityCheck);

  useEffect(() => {
    refreshAccessibility();
    refreshDotfiles();
  }, [refreshAccessibility, refreshDotfiles]);

  function nextStep() {
    setStep(step + 1);
  }

  function finish() {
    refreshAccessibility();
    refreshDotfiles();
    Internal.sendOnboardingRequest({
      action: Fig.OnboardingAction.FINISH_ONBOARDING,
    })
      .then(() => {
        refreshLocalState().catch((err) => console.error(err));
      })
      .catch((err) => console.error(err));
  }

  function skipInstall() {
    if (!check.id) return;

    if (check.id === "dotfiles") {
      setDotfiles(true);
      setStep(step + 1);
    }

    if (check.id === "accessibility") {
      setAccessibility(true);
      setStep(step + 1);
    }
  }

  // Past the last step (including skipped platform-restricted steps)
  const done = step >= onboarding.length;
  useEffect(() => {
    if (done) finish();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [done]);

  if (done) return null;

  if (
    platformInfo &&
    !matchesPlatformRestrictions(platformInfo, check.platformRestrictions)
  ) {
    setStep(step + 1);
  }

  if (check.id === "welcome") {
    return <WelcomeModal next={nextStep} />;
  }

  if (INSTALL_STEP_IDS.includes(check.id)) {
    return <InstallModal check={check} skip={skipInstall} next={nextStep} />;
  }

  return null;
}

export function WelcomeModal({ next }: { next: () => void }) {
  return (
    <div className="flex flex-col items-center gap-8 gradient-q-secondary-light -m-10 p-4 pt-10 rounded-lg text-white">
      <div className="flex flex-col items-center gap-8">
        <img src={appIcon} alt="Tab" className="w-24" />
        <div className="flex flex-col gap-2 items-center text-center">
          <h2 className="text-2xl text-white font-semibold select-none leading-none font-heading tracking-tight">
            Welcome!
          </h2>
          <p className="text-sm">Let's get you set up...</p>
        </div>
      </div>
      <div className="flex flex-col items-center gap-2 text-white text-sm font-bold">
        <Button variant="glass" onClick={() => next()} className="flex gap-4">
          Get started
        </Button>
      </div>
    </div>
  );
}

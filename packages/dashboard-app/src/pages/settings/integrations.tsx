import { UserPrefView } from "@/components/preference/list";
import settings from "@/data/integrations";

export default function Page() {
  return (
    <UserPrefView
      array={settings}
      className="w-auto self-start [&>section]:grid [&>section]:grid-cols-1 [&>section]:md:grid-cols-2 [&>section]:xl:grid-cols-3 [&>section]:2xl:grid-cols-4 [&>section>h2]:col-span-full"
    />
  );
}

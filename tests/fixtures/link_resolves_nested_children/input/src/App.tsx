import { PageShell } from "./PageShell";
import { ProfilePage } from "./ProfilePage";

export function App() {
    return (
        <PageShell>
            <ProfilePage />
            <LocalBadge />
        </PageShell>
    );
}

function LocalBadge() {
    return <div />;
}

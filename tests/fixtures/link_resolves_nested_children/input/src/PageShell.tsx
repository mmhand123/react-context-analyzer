import { GlobalNav } from "./GlobalNav";

export function PageShell({ children }) {
    return (
        <main>
            <ShellFrame />
            {children}
            <GlobalNav />
        </main>
    );
}

function ShellFrame() {
    return <div />;
}

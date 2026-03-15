import { createContext, useContext } from "react";

const AuthContext = createContext(null);

export function App() {
  const auth = useContext(AuthContext);

  return (
    <AuthContext.Provider value={auth}>
      <ProfilePage />
    </AuthContext.Provider>
  );
}

function ProfilePage() {
  return <Header />;
}

import { BrowserRouter, Route, Routes } from "react-router-dom";
import { Layout } from "./components/Layout";
import { Missing } from "./components/Missing";
import { DetailPage } from "./pages/DetailPage";
import { FormPage } from "./pages/FormPage";
import { ListPage } from "./pages/ListPage";

export function App() {
  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<ListPage />} />
          <Route path="/reviews/new" element={<FormPage mode="create" />} />
          <Route path="/reviews/:id/edit" element={<FormPage mode="edit" />} />
          <Route path="/reviews/:id" element={<DetailPage />} />
          <Route
            path="*"
            element={<Missing title="ページが見つかりません" body="アドレスを確認してください。" />}
          />
        </Routes>
      </Layout>
    </BrowserRouter>
  );
}

<script lang="ts">
  import { Router, Link, Route } from "svelte-routing";

  import About from "./routes/About.svelte";
  import Posts from "./routes/Posts.svelte";
  import Me from "./routes/Me.svelte";

  export let url:string = "";
  const about:string = "MOSS";

</script>

<main>

  <Router {url}>
    <nav>
      <Link to="/">About</Link> | 
      <Link to="/Posts">Posts</Link> | 
      <Link to="/Me">Me</Link>
    </nav>
    <div>
      <h2>{ about }</h2>
      <Route path="/"><About/></Route>
      <Route path="/Posts"><Posts/></Route>
      <Route path="/Me"><Me/></Route>
    </div>
  </Router>

</main>
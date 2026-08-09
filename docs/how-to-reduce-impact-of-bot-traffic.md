PatronView

PUBLISHED AUG 07, 2026

# 99% of My Website Traffic Is Bots

A year of fighting scrapers on my 1.5 million-page website. What I tried, how I failed, and what's working now.

August 7, 2026 * 27 MIN READ * `general`

<figure class="fl-hero-figure">
<img src="/images/blog/bots/only-human-hero.jpg" loading="eager" fetchpriority="high" decoding="async" width="1200" height="630" alt="A crowd of hand-drawn robots with a single blue human waving in the middle, labelled the only human" />
<figcaption>That's me waving to you because blue is one of my favorite colors.</figcaption>
</figure>

If you're trying to steal my data: start here! I'll tell you exactly what firewall rules I have in place so you can work around them more easily.

I'm sort of joking. I don't even know if a web crawler reads blog posts to get tips like this.

But I've spent a year fighting bots on my philanthropy donor database, [PatronView](https://patronview.com). Nobody teaches you this stuff when you're vibe coding, so I'll share what I've learned.

Some of what's below:

- My site got **3.6 million requests** from Chinese bots in just one day.
- Cloudflare says Anthropic's crawlers run about 3,000 crawls per visitor referred. I measured mine at **35,000:1**.
- Two days ago I blocked Amazon's AI search crawler. It was reading **117,000 pages a day** and has never sent me a single visitor.
- My CAPTCHA solve rate is **0.24%**. The bots don't even try.
- And every firewall rule I run is listed at the bottom.

## 214 bot page loads for every 1 human

In the week I published this post, [my server](https://patronview.com/) answered **2.5 million requests** from the outside world and served **1.28 million full pages**. But my visitor stats only recorded 5,977 pageviews.

So for every page load I see, about 214 more happen that I never see.

That's where the title comes from. 5,977 human pageviews out of 1.28 million pages served is less than half of one percent. If anything, calling it 99% bots is me rounding down.

<figure class="bp-chart">

<figcaption>The blue square is the traffic I can see. The gray squares are what actually pounds my server. I use self-hosted <a href="https://github.com/plausible/community-edition/">Plausible Community Edition</a> for analytics, and it only counts visitors who run JavaScript. Almost no bot does.</figcaption>
</figure>

My visitor stats show a tidy little site, about 500 visitors a day. But my server answers millions of real requests every week.

If you only look at visitor stats from a JavaScript tool like **Plausible** or **Fathom** or **Google Analytics**, you have no idea what's hitting your server. I didn't.

## The first botnet

PatronView is a custom database of American philanthropists that I cobbled together with various levels of intelligence. It has 1.5 million individual profile pages built from IRS 990 forms, public donor walls, and annual reports.

And yes, my site gets its data by scraping those public documents. So I'm a scraper writing a blog post complaining about scrapers. I'm aware of how that sounds. For the record, I scrape my sources a few times a year, not thousands of times a day.

From the beginning, SEO crawlers like `SemrushBot`, `AhrefsBot`, `MJ12bot`, and `DataForSEOBot` pounded the site, crawling every single page over and over. Blocking them in `robots.txt` and then at the Cloudflare level with security rules was my gateway drug into all of this.

Then in November 2025, four thousand "visitors" showed up over a few days. Each visited exactly one page with a bounce rate of 99%. More telling was that they had no referrer, which is **usually the easiest way I spot a bot**. And they were only crawling my fund pages (like [this one](https://patronview.com/funds/metropolitan-museum-of-art-the-met-board-of-trustees), [this one](https://patronview.com/funds/menil-collection-board-of-trustees), and [this one](https://patronview.com/funds/george-eastman-museum-designated-giving-house-garden-restoration-2)), which only 10% of real visitors ever touch. But those 4,000 bots were just the warm-up.

## The day China arrived

On April 22, my site took **3.6 million requests in a single day**. They came from 361,844 unique IP addresses, and nearly all of them were in China.

<figure>
<img src="/images/blog/bots/china-dragon.jpg" loading="lazy" decoding="async" width="1058" height="861" alt="A hand-drawn Chinese dragon coiled around a neoclassical museum building marked PatronView. The dragon&#39;s entire body is made of hundreds of small red robots linked nose to tail, some carrying tiny red flags. A single small human in a blue outfit stands on the steps below." />
<figcaption>I tried to imagine what all those Chinese bots looked like. With apologies for the cliche red dragon!!</figcaption>
</figure>

Cloudflare's Managed Challenge (their invisible CAPTCHA) absorbed 1.18 million of those in the first ten hours. And yet an alarming number of these Chinese bots were passing the challenge.

<figure class="bp-chart">

<figcaption>Requests per day in April 2026. I implemented the China-wide block on April 23rd and immediately saw relief.</figcaption>
</figure>

<figure>
<img src="/images/blog/bots/China-traffic.png" loading="lazy" decoding="async" width="830" height="432" alt="Cloudflare security events chart totalling 1.18M managed challenges. The line sits flat at zero until about 15:00, then jumps to a jagged band between 14,000 and 25,000 events per interval and stays there overnight." />
<figcaption>Cloudflare security events during the flood: 1.18M managed challenges in about 10 hours.</figcaption>
</figure>

So I did the thing you're not supposed to do: **I blocked the entire country of China at the edge.** A few days later, a copycat wave from Vietnam started, so I blocked Vietnam, too. And then Singapore.

My site is a database of American museum donors, in English. My real search traffic is 95.9% United States and 1.3% Canada.

When I [tweeted about the flood](https://x.com/nickgraynews/status/2046939483933725075) of Chinese web traffic, the replies told me everyone is fighting this:

- **Matt Paulson** from MarketBeat: "Don't forget to add Russia to the list."
- **Jack Ellis** from Fathom Analytics: "We saw a ton of spam traffic from China over the last 6 months. I will say that customers have seen China drop completely now."
- **Jeremy Brandt**: "Our country block list on Cloudflare for all of our sites is... extensive."
- **Chris Lewicki**: "I decided to stop fighting it. My counter-measures were introducing too much complexity in my life."
- **Rodrigo Rocco**: "It's been crazy lately, they are using thousands of residential IPs making only 1 call each, so hard to stop it."

Remember that last one from Rodrigo, because it comes back to bite me later.

## The Claude ratio

Cloudflare has said Anthropic's crawlers run at about 3,000 pages crawled per visitor referred. In June, I measured mine: **35,000 to 1**.

To be clear about what that means: for every one visitor Claude sent me, its crawler read 35,000 of my pages.

<figure>
<img src="/images/blog/bots/claude-ratio.jpg" loading="lazy" decoding="async" width="1106" height="846" alt="A hand-drawn robot walking away while towing a cart stacked with a tower of paper so tall it runs off the top of the frame, labelled 35,000 pages. On its other open palm it holds one tiny human in a blue outfit, labelled 1 visitor." />
<figcaption>I didn't use the Claude logo so they can't sue me for copyright! But imagine this bot's name is Claude.</figcaption>
</figure>

I found it in Cloudflare's AI crawler dashboard, shown below. `Claude-SearchBot`, Anthropic's search crawler, had requested 420,680 pages in one week. That same week, Claude sent me 12 human visitors, measured by `Claude-User`, the separate user-agent Anthropic sends when a real person asks Claude to fetch a page. That's the flat line at the bottom.

<figure>
<img src="/images/blog/bots/Claude-SearchBot0.jpeg" loading="lazy" decoding="async" width="1314" height="944" alt="Cloudflare AI crawler dashboard for one week in June. Claude-SearchBot: 420.68k requests, a dense line oscillating between roughly 1,500 and 3,500 per hour. Claude-User: 12, a flat line pinned to the bottom axis." />
</figure>

The bandwidth tab was worse. That week I served 4.63 GB to the bot, and 175 KB to the humans it sent me:

<figure>
<img src="/images/blog/bots/Claude-SearchBot.jpeg" loading="lazy" decoding="async" width="1312" height="968" alt="The same dashboard on the bandwidth tab. Claude-SearchBot: 4.63 GB, a line running between roughly 20 MB and 40 MB per hour all week. Claude-User: 175.56 kB, flat against the bottom axis." />
</figure>

I love Anthropic and Claude as much as the next guy. My website was built with Claude Code, and I used Claude Code to help me refine my security rules on the Cloudflare API. Complaining about Claude scraping the site that I used Claude to build is a level of irony I've made peace with.

But Anthropic and Claude do not send me traffic. So I blocked `Claude-SearchBot` at the firewall.

And they respected it! The crawler dropped from 60,000 requests a day to about 25 attempts a day. The polite AI companies really do take the 403. (A 403 is the web's "access denied" response.)

## Pages crawled per visitor referred

That block gave me the metric I now use for everything: **pages crawled per visitor referred**. Google earns its keep. Bing is 9x worse but still defensible. The AI crawlers that are training LLMs are not close:

<figure class="bp-chart">

<figcaption>Crawl-to-referral ratios I measured. Google: 46 crawls per visitor sent. Amazon: infinity.</figcaption>
</figure>

About that Amazon bar. While researching this post, I found that `Amzn-SearchBot`, Amazon's AI search crawler, is now my \#1 crawler at about **117,000 requests per day**. That's nearly double Claude's pace from June. It feeds Rufus and Alexa answers, so it will never send web traffic and get people to sign up for my newsletter, and I doubt it will even give me attribution or backlinks.

So two days ago I blocked it the same way. It took two minutes.

Cloudflare's managed AI Crawl Control does a lot of heavy lifting here too. It blocks the declared training bots (`GPTBot`, `ClaudeBot`, `CCBot`, `Bytespider`) before my rules even run.

That same dashboard shows Bingbot at 158,610 requests for 680 visitors. My Bing referrals are actually growing, so I'm happy for them to keep scraping. And it shows Anthropic down to 47 requests since my block. (`AI-Crawl-Control-crawlers.png` of the crawler dashboard.)

## The American datacenter wave

In July, someone new came knocking or else those scrapers adapted to my geography rules. They came from the United States instead.

A steady wave of headless Chrome browsers on AWS. These ran JavaScript and polluted my stats with thousands of fake "visitors."

<figure>
<img src="/images/blog/bots/museum-swarm.jpg" loading="lazy" decoding="async" width="1314" height="768" alt="A hand-drawn museum building with blue and gray robots swarming it from every direction, climbing the columns and sitting on the steps, with arrows showing more pouring in from both sides. One oversized human sits alone on the roof looking overwhelmed." />
<figcaption>Me, as a cartoon, on top of a museum that's getting attacked by robots.</figcaption>
</figure>

The fix was the most durable rule I've built: **challenge every datacenter**.

Real readers browse from Comcast and T-Mobile, and almost never from an `AWS us-east-1` IP address. The exceptions are people on cloud desktops or VPNs, which is why this rule is a challenge instead of a block.

## The day I turned off the CAPTCHA

Cloudflare's "JavaScript Detections" feature had been injecting a challenge script into every page, all year. I guess I turned it on at some point? And maybe it was supposed to be helping? But then I found it this week while trying to improve my site speed scores.

That script cost **2,875 milliseconds** on a mid-range phone. My entire site's own JavaScript runs in 278ms. It was the single biggest reason my mobile Lighthouse score was 58, and on my Cloudflare plan its verdict isn't even readable by firewall rules. So I was taking a 40-point performance tax for telemetry nobody could read.

On August 5, I turned it off and hit a **Lighthouse score of 99** within an hour.

Five hours later, a scraper on Microsoft Azure took 23,000 pages in one hour, from more than 80 IP addresses, each one politely staying under my rate limit. So I guess the JavaScript Detections was working. But then I turned it off, and needed a new plan.

And a botnet of residential IPs showed up pretending to be Chrome 118 through 120. Browser versions from 2023, frozen in whatever scraping toolkit they downloaded.

## The residential botnets

A residential botnet is scraping traffic routed through thousands of ordinary home internet connections, rented by the request from proxy companies. Every request looks like a real person on a real ISP.

<figure>
<img src="/images/blog/bots/residential-masks.jpg" loading="lazy" decoding="async" width="1162" height="628" alt="A hand-drawn row of five ordinary front doors. On each doorstep stands a small robot holding up a cardboard cutout of the same smiling human face on a stick. The robot at the far end has lowered its mask, showing the robot underneath. A man in a blue outfit watches from the pavement." />
<figcaption>These residential botnets are wild. Worth reading up on if you've never run into one!</figcaption>
</figure>

I'd seen this shape before. Back in November 2025, I noticed something strange in my visitor stats: dozens of countries visiting my site in almost identical numbers on the same day. Here's the screenshot from my analytics:

<figure>
<img src="/images/blog/bots/November2025-post.jpeg" loading="lazy" decoding="async" width="1280" height="1168" alt="Analytics table of visitors by country. United States 640 (31.9%), Germany 215, Vietnam 100, Russia 91, Italy 77, Canada 74, UK 70, then a long tail where roughly a dozen countries - Indonesia, Japan, Chile, Brazil, Hong Kong, Lithuania, Malaysia, South Africa - all sit within a few visitors of each other around 33 to 36." />
<figcaption>The country breakdown from <a href="https://x.com/nickgraynews/status/1994793007418155464">my November 2025 tweet</a>. Look at the spread: Indonesia, Japan, Chile, Brazil, Hong Kong, almost evenly split.</figcaption>
</figure>

That even spread across forty countries is the signature of a rented proxy network: thousands of real-looking connections, all working for the same customer. It's exactly what Rodrigo warned about in those tweet replies.

Nine months later, the same shape came back, this time on American home internet connections. Both of my best rules sort traffic by where it comes from. One challenges anything outside North America. The other challenges anything from a big cloud provider. This traffic is neither. It arrives from a house in Ohio on Spectrum, so both rules wave it through. You can see it ramp up in this chart of unique IP addresses hitting my site each day in July:

<figure class="bp-chart">

<figcaption>Unique IPs hitting my site per day, ramping through July. Nothing about the site changed in July.</figcaption>
</figure>

I responded with two new rules. First, Azure and the other big clouds joined the datacenter challenge list.

And I added my favorite dumb rule: **challenge browsers frozen years in the past**. Chrome 100 through 130 gets a CAPTCHA. So does old Firefox. I checked my real traffic first: only 0.54% of my actual search visitors run browsers that old, and most of those are on Firefox 115 ESR, which I exempted. The frozen botnet fails this rule all day long.

## My Cloudflare Security rules

Everything runs on Cloudflare's WAF. I pay for the Pro plan at $25 per month. The logic ports to any modern firewall. Here are the rules I'm running today:

1.  **Block China and Vietnam.** See above. Country blocks add zero latency for everyone else.
2.  **Block 12 SEO crawlers by user-agent.** Semrush, Ahrefs, MJ12bot, DotBot, BLEXBot, Barkrowler, and friends. They identify themselves honestly, bless them.
3.  **Block AI crawlers with bad ratios by user-agent.** Right now: `Claude-SearchBot` and `Amzn-SearchBot`. They can still read `robots.txt`, so they know they're not welcome. My `robots.txt` separately tells `GPTBot`, `ClaudeBot`, `CCBot`, `Bytespider`, and the other declared training bots to go away. The polite ones comply.
4.  **Skip everything below for verified bots.** Cloudflare cryptographically verifies Googlebot, Bingbot, and Applebot. The ordering is the trick: this skip sits after my block rules. A "verified" bot I've blocked stays blocked, but no challenge ever touches Google.
5.  **Challenge every continent except North America.** The blunt one, and the one I'd defend hardest. My audience is 97% North America. Everyone else gets one invisible CAPTCHA per 45 minutes.
6.  **Challenge empty user-agents.** No real human's browser sends an empty user-agent string.
7.  **Challenge 46 datacenter ASNs.** Humans don't browse from AWS. (Generally.)
8.  **Challenge stale browsers.** The frozen botnet rule.
9.  **Rate limit: 30 page requests per 10 seconds per IP.** Static files excluded, so it never fires on a normal page load.

My `robots.txt` also blocks all 12 SEO crawlers by name, and Cloudflare adds its own AI-training blocks on top of it at the edge. You can read the live file at [`patronview.com/robots.txt`](https://patronview.com/robots.txt).

<figure>
<img src="/images/blog/bots/403-wall.jpg" loading="lazy" decoding="async" width="1200" height="702" alt="A hand-drawn brick wall with a crowd of robots stretching back into the distance, all pressed against it with their arms raised. A small sign on the bricks reads 403. A man in a blue outfit has already walked through a door in the wall and is strolling away on the other side." />
<figcaption>I thought it would be funny to see all the robots like this, but now I almost feel bad for them. They're sorta cute.</figcaption>
</figure>

Does all this challenging hurt real humans? Barely.

Over a recent 48 hour window, Cloudflare issued 106,437 challenges. **252 were solved.** That's a 0.24% solve rate.

<figure class="bp-chart">

<figcaption>Singapore: 7,089 challenged, 3 solved. I hope those three found what they needed.</figcaption>
</figure>

## My server costs

The site runs on Cloudflare Workers with the D1 database and the KV cache at the edge. Most bot requests hit cache and cost fractions of a penny. My normal bill for running this whole site is around $90 a month. During one bad spike month, it jumped about 500%.

The bots use 99% of the bill and I pay 100% of it.

The real costs were the sneaky ones:

- **My visitor stats got so polluted I couldn't trust my own numbers.** This was the one that hurt. I'm trying to run a business here. I want to know what real people read on my site so I know what to build next. I couldn't see them through the bots.
- One afternoon, cache misses from the Azure burst piled up and real visitors got 504 errors.

If I were on a traditional VPS paying for CPU and bandwidth, millions of weekly requests where over 99% are bots would be an existential problem. On an edge platform, it's a nuisance.

Some of that bill is my own fault, to be clear. I'm still cleaning up inefficient queries and cache settings that made those spike months worse. I keep thinking about switching to a VPS and hosting my own local database, but I've resisted because I like playing with the Cloudflare platform.

I think that asymmetry is why scraping keeps getting worse. The scrapers' costs fell faster than everyone's defenses improved.

## What to do if this is happening to you

1.  **Look at your server logs, not your visitor stats.** Your stats see less than half a percent of what's happening.
2.  **Block by ASN, not by IP.** IPs rotate forever. Networks don't.
3.  **Challenge, don't block, when humans might be inside.** A managed challenge is invisible to most real people. And it lets you measure.
4.  **Watch your solve rates.** A 0.2% solve rate means bots, so keep the rule. A 30% solve rate means you're taxing humans, so fix the rule.
5.  **Profile what your security features cost.** My CAPTCHA script was more expensive than my entire site.

## Conclusion

So did any of it work?

In the 24 hours since I finished these rules, Cloudflare blocked **46,729 requests** outright. 43,150 of those were Amazon's crawler alone, still hammering the block I set up two days ago.

It also issued 63,969 challenges, and only 552 of them were solved.

So the rules are working. Hopefully they keep letting through the bots I actually want, and the humans. At least for now.

Let me tell you, this feels like whack-a-mole. Every few weeks I have to spend time fighting somebody new away.

<figure>
<img src="/images/blog/bots/whack-a-bot.jpg" loading="lazy" decoding="async" width="767" height="792" alt="A hand-drawn arcade cabinet lettered whack-a-bot with a small orange Cloudflare cloud beside the name. Robot heads pop up out of holes all over the board. A man in a blue outfit, seen from behind, swings a mallet at one of them." />
<figcaption>I love this image. And Skee-Ball is actually my favorite arcade game, in case you were wondering.</figcaption>
</figure>

There's a real conflict here. I want Google and Bing and DuckDuckGo to crawl my site and send me new readers. But I don't want everyone else strip-mining it. And the residential botnets already solve CAPTCHAs and look exactly like real browsers. Blocking them at the network level is impossible by design.

Scraping keeps getting worse because it keeps getting cheaper, so I think the only real fix is economic. Cloudflare is building [pay-per-crawl](https://blog.cloudflare.com/introducing-pay-per-crawl/), where crawlers pay per request at the edge. I'd happily sell Amazon those 3.5 million pages a month at a fair rate.

Until a market like that exists, my rule is simple: **a crawler that never sends me a visitor gets blocked.**

If you're fighting this too, email me: <hello@nickgray.net>. I'd love to compare notes.

## Appendix: the exact rule expressions

For the technical crowd. These are the actual expressions behind the rules above, copied out of my Cloudflare zone via the API. They're written in Cloudflare's [rules language](https://developers.cloudflare.com/ruleset-engine/rules-language/) and should port to most firewalls with light translation. I've omitted a couple of internal housekeeping skips.

**Block China and Vietnam.** Two separate rules, action: Block.

    (ip.src.country in {"CN"})
    (ip.src.country eq "VN")

**Block SEO crawlers by user-agent.** Action: Block.

    (lower(http.user_agent) contains "barkrowler") or
    (lower(http.user_agent) contains "thinkbot") or
    (lower(http.user_agent) contains "brightbot") or
    (lower(http.user_agent) contains "mj12bot") or
    (lower(http.user_agent) contains "semrushbot") or
    (lower(http.user_agent) contains "siteauditbot") or
    (lower(http.user_agent) contains "ahrefsbot") or
    (lower(http.user_agent) contains "ahrefssiteaudit") or
    (lower(http.user_agent) contains "dataforseobot") or
    (lower(http.user_agent) contains "dotbot") or
    (lower(http.user_agent) contains "blexbot") or
    (lower(http.user_agent) contains "splitsignalbot")

**Block AI crawlers with bad ratios.** Action: Block. Note the carve-out: a blocked crawler can still read `robots.txt`, the file that tells it why it's blocked.

    (http.request.uri.path ne "/robots.txt" and
      (http.user_agent contains "Claude-SearchBot" or
       http.user_agent contains "Amzn-SearchBot"))

**Skip everything below for verified bots.** Action: Skip (remaining custom rules + rate limiting). One expression. Order matters here: it sits after the blocks above, so a blocked bot stays blocked, but Googlebot never sees a challenge.

    (cf.client.bot)

**Challenge every continent except North America.** Action: Managed Challenge. The country carve-out at the end is because Guam, American Samoa, and the Northern Mariana Islands are US territories that carry Oceania continent codes. Real American readers live there. `T1` is the Tor network.

    ((ip.src.asnum in {212238 139341 9009}) or
     (ip.src.continent in {"AF" "AN" "AS" "OC" "SA" "T1" "EU"}))
    and not cf.client.bot
    and not (ip.src.country in {"GU" "AS" "MP"})

**Challenge empty user-agents.** Action: Managed Challenge.

    (http.user_agent eq "") and not cf.client.bot

**Challenge datacenter and cloud ASNs.** Action: Managed Challenge. AWS, Azure, Google Cloud, Oracle, Alibaba, DigitalOcean, Vultr, Linode, OVH, Hetzner, Hurricane Electric, and every hosting network I've caught scraping.

    (ip.src.asnum in {14618 16509 8987 396982 14061 20473 63949
      16276 24940 213230 55286 64286 36352 18779 40676 397423
      46261 399073 2914 13332 30058 11798 21769 62874 202015
      55933 393886 27411 396362 395954 396190 19148 30633 8075
      8070 8068 8069 31898 45102 37963 36351 136907 45090 207990
      17497 6939}) and not cf.client.bot

**Challenge stale browser majors.** Action: Managed Challenge. Cloudflare's Pro plan has no regex matching in rule expressions (that's a Business feature), so yes, this is 55 `contains` clauses. Firefox 115 is skipped because it's the old ESR release my real visitors still run, stranded on older operating systems. It needs a bump every quarter as versions age.

Expand the full expression

    (http.user_agent contains "Chrome/100." or
     http.user_agent contains "Chrome/101." or
     http.user_agent contains "Chrome/102." or
     http.user_agent contains "Chrome/103." or
     http.user_agent contains "Chrome/104." or
     http.user_agent contains "Chrome/105." or
     http.user_agent contains "Chrome/106." or
     http.user_agent contains "Chrome/107." or
     http.user_agent contains "Chrome/108." or
     http.user_agent contains "Chrome/109." or
     http.user_agent contains "Chrome/110." or
     http.user_agent contains "Chrome/111." or
     http.user_agent contains "Chrome/112." or
     http.user_agent contains "Chrome/113." or
     http.user_agent contains "Chrome/114." or
     http.user_agent contains "Chrome/115." or
     http.user_agent contains "Chrome/116." or
     http.user_agent contains "Chrome/117." or
     http.user_agent contains "Chrome/118." or
     http.user_agent contains "Chrome/119." or
     http.user_agent contains "Chrome/120." or
     http.user_agent contains "Chrome/121." or
     http.user_agent contains "Chrome/122." or
     http.user_agent contains "Chrome/123." or
     http.user_agent contains "Chrome/124." or
     http.user_agent contains "Chrome/125." or
     http.user_agent contains "Chrome/126." or
     http.user_agent contains "Chrome/127." or
     http.user_agent contains "Chrome/128." or
     http.user_agent contains "Chrome/129." or
     http.user_agent contains "Chrome/130." or
     http.user_agent contains "Firefox/100." or
     http.user_agent contains "Firefox/101." or
     http.user_agent contains "Firefox/102." or
     http.user_agent contains "Firefox/103." or
     http.user_agent contains "Firefox/104." or
     http.user_agent contains "Firefox/105." or
     http.user_agent contains "Firefox/106." or
     http.user_agent contains "Firefox/107." or
     http.user_agent contains "Firefox/108." or
     http.user_agent contains "Firefox/109." or
     http.user_agent contains "Firefox/110." or
     http.user_agent contains "Firefox/111." or
     http.user_agent contains "Firefox/112." or
     http.user_agent contains "Firefox/113." or
     http.user_agent contains "Firefox/114." or
     http.user_agent contains "Firefox/116." or
     http.user_agent contains "Firefox/117." or
     http.user_agent contains "Firefox/118." or
     http.user_agent contains "Firefox/119." or
     http.user_agent contains "Firefox/120." or
     http.user_agent contains "Firefox/121." or
     http.user_agent contains "Firefox/122." or
     http.user_agent contains "Firefox/123." or
     http.user_agent contains "Firefox/124.")
    and not cf.client.bot

**Rate limit.** Action: Managed Challenge when an IP exceeds 30 matching requests in 10 seconds. It only counts extensionless paths, so the 40 images and stylesheets on a normal page load never trip it. (Strictly, the counter is per IP per Cloudflare data center, so a distributed botnet can slide under it. It catches the lazy ones.)

    (http.request.uri.path.extension eq "") and not cf.client.bot

**And one bonus rule I keep disabled: the panic button.** Same expression as the rate limit, but as a plain Managed Challenge on every page request from anything that isn't a verified bot. When a scrape burst is actively hammering the site, I flip it on, let the flood die, and flip it off within the hour.

<style>
.blog-prose .bp-aside{background:#fcfcfb;border-left:3px solid #2a78d6;border-radius:0 8px 8px 0;padding:10px 14px;font-style:italic}
.blog-prose figure svg{width:100%;height:auto;display:block}
.blog-prose figure svg text{font-family:system-ui,-apple-system,"Segoe UI",sans-serif}
.blog-prose .bp-grid{stroke:#e1e0d9;stroke-width:1}
.blog-prose .bp-ax{fill:#898781;font-size:12px}
.blog-prose .bp-lbl{fill:#0b0b0b;font-size:13px}
.blog-prose .bp-val{fill:#0b0b0b;font-size:13px;font-weight:600}
.blog-prose .bp-val-inv{fill:#fff;font-size:12px;font-weight:600}
.blog-prose .bp-ann{fill:#52514e;font-size:12.5px;font-weight:600}
.blog-prose .bp-bar{fill:#2a78d6}
.blog-prose .bp-bar-hot{fill:#e34948}
.blog-prose .bp-line{fill:none;stroke:#2a78d6;stroke-width:2}
.blog-prose .bp-area{fill:#2a78d6;opacity:.13}
.blog-prose .bp-dot{fill:#2a78d6}
.blog-prose .bp-waffle{fill:#e1e0d9}
.blog-prose .bp-waffle-hot{fill:#2a78d6}
.blog-prose .bp-ttitle{fill:#0b0b0b;font-size:14.5px;font-weight:700}
</style>

CONTENTS

1.  <a href="#214-bot-page-loads-for-every-1-human" data-toc-link=""><span class="n mono">01</span><span>214 bot page loads for every 1 human</span></a>
2.  <a href="#the-first-botnet" data-toc-link=""><span class="n mono">02</span><span>The first botnet</span></a>
3.  <a href="#the-day-china-arrived" data-toc-link=""><span class="n mono">03</span><span>The day China arrived</span></a>
4.  <a href="#the-claude-ratio" data-toc-link=""><span class="n mono">04</span><span>The Claude ratio</span></a>
5.  <a href="#pages-crawled-per-visitor-referred" data-toc-link=""><span class="n mono">05</span><span>Pages crawled per visitor referred</span></a>
6.  <a href="#the-american-datacenter-wave" data-toc-link=""><span class="n mono">06</span><span>The American datacenter wave</span></a>
7.  <a href="#the-day-i-turned-off-the-captcha" data-toc-link=""><span class="n mono">07</span><span>The day I turned off the CAPTCHA</span></a>
8.  <a href="#the-residential-botnets" data-toc-link=""><span class="n mono">08</span><span>The residential botnets</span></a>
9.  <a href="#my-cloudflare-security-rules" data-toc-link=""><span class="n mono">09</span><span>My Cloudflare Security rules</span></a>
10. <a href="#my-server-costs" data-toc-link=""><span class="n mono">10</span><span>My server costs</span></a>
11. <a href="#what-to-do-if-this-is-happening-to-you" data-toc-link=""><span class="n mono">11</span><span>What to do if this is happening to you</span></a>
12. <a href="#conclusion" data-toc-link=""><span class="n mono">12</span><span>Conclusion</span></a>
13. <a href="#appendix-the-exact-rule-expressions" data-toc-link=""><span class="n mono">13</span><span>Appendix: the exact rule expressions</span></a>

RELATED POSTS

<a href="/news/spring-cleaning-redesign/" data-astro-prefetch="hover"><span class="fl-relatedlist-title">Spring Cleaning and a Redesign</span> <span class="fl-relatedlist-date mono"> MAR 25, 2026 </span></a> <a href="/news/updated-database-now-live/" data-astro-prefetch="hover"><span class="fl-relatedlist-title">Updated Database Now Live</span> <span class="fl-relatedlist-date mono"> AUG 14, 2025 </span></a> <a href="/news/database-updates-on-hold/" data-astro-prefetch="hover"><span class="fl-relatedlist-title">Database Updates on Hold</span> <span class="fl-relatedlist-date mono"> JUL 04, 2025 </span></a> <a href="/news/welcome-to-patron-view/" data-astro-prefetch="hover"><span class="fl-relatedlist-title">Welcome to PatronView</span> <span class="fl-relatedlist-date mono"> JUN 15, 2025 </span></a>

CONTENTS

1.  <a href="#214-bot-page-loads-for-every-1-human" data-toc-link=""><span class="n mono">01</span><span>214 bot page loads for every 1 human</span></a>
2.  <a href="#the-first-botnet" data-toc-link=""><span class="n mono">02</span><span>The first botnet</span></a>
3.  <a href="#the-day-china-arrived" data-toc-link=""><span class="n mono">03</span><span>The day China arrived</span></a>
4.  <a href="#the-claude-ratio" data-toc-link=""><span class="n mono">04</span><span>The Claude ratio</span></a>
5.  <a href="#pages-crawled-per-visitor-referred" data-toc-link=""><span class="n mono">05</span><span>Pages crawled per visitor referred</span></a>
6.  <a href="#the-american-datacenter-wave" data-toc-link=""><span class="n mono">06</span><span>The American datacenter wave</span></a>
7.  <a href="#the-day-i-turned-off-the-captcha" data-toc-link=""><span class="n mono">07</span><span>The day I turned off the CAPTCHA</span></a>
8.  <a href="#the-residential-botnets" data-toc-link=""><span class="n mono">08</span><span>The residential botnets</span></a>
9.  <a href="#my-cloudflare-security-rules" data-toc-link=""><span class="n mono">09</span><span>My Cloudflare Security rules</span></a>
10. <a href="#my-server-costs" data-toc-link=""><span class="n mono">10</span><span>My server costs</span></a>
11. <a href="#what-to-do-if-this-is-happening-to-you" data-toc-link=""><span class="n mono">11</span><span>What to do if this is happening to you</span></a>
12. <a href="#conclusion" data-toc-link=""><span class="n mono">12</span><span>Conclusion</span></a>
13. <a href="#appendix-the-exact-rule-expressions" data-toc-link=""><span class="n mono">13</span><span>Appendix: the exact rule expressions</span></a>

<span class="num mono">-</span>

## More on this subject

<span class="more mono">4POSTS</span>

<a href="/news/spring-cleaning-redesign/" class="fl-row fl-row-link" data-astro-prefetch="hover"><span class="fl-row-body"><span class="fl-row-title">Spring Cleaning and a Redesign</span></span><span class="fl-row-date mono">MAR 25, 2026</span></a><a href="/news/updated-database-now-live/" class="fl-row fl-row-link" data-astro-prefetch="hover"><span class="fl-row-body"><span class="fl-row-title">Updated Database Now Live</span></span><span class="fl-row-date mono">AUG 14, 2025</span></a><a href="/news/database-updates-on-hold/" class="fl-row fl-row-link" data-astro-prefetch="hover"><span class="fl-row-body"><span class="fl-row-title">Database Updates on Hold</span></span><span class="fl-row-date mono">JUL 04, 2025</span></a><a href="/news/welcome-to-patron-view/" class="fl-row fl-row-link" data-astro-prefetch="hover"><span class="fl-row-body"><span class="fl-row-title">Welcome to PatronView</span></span><span class="fl-row-date mono">JUN 15, 2025</span></a>

`news` `general` `patrons` `institutions`

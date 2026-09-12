"""Stable local-stack account selectors for ordinary Live Demo product data."""

import dataclasses


@dataclasses.dataclass(frozen=True)
class SeededAccount:
	"""One fixed Product Role mapping for deployment-gated demo entry."""

	setting: str
	account_id: str
	product_role: str


SEEDED_ACCOUNTS = (
	SeededAccount("PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID", "00000000-0000-0000-0000-000000000101", "instructor"),
	SeededAccount("PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID", "00000000-0000-0000-0000-000000000102", "student"),
	SeededAccount("PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID", "00000000-0000-0000-0000-000000000103", "student"),
	SeededAccount("PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID", "00000000-0000-0000-0000-000000000104", "student"),
	SeededAccount("PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID", "00000000-0000-0000-0000-000000000105", "sysadmin"),
)

SEEDED_STUDENT_AUTHENTICATION_EMAILS = (
	("00000000-0000-0000-0000-000000000102", "mary.okafor@live-demo.invalid"),
	("00000000-0000-0000-0000-000000000103", "jack.nguyen@live-demo.invalid"),
	("00000000-0000-0000-0000-000000000104", "avery.thompson@live-demo.invalid"),
)

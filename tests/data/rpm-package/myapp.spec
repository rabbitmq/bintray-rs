Name:     myapp
Version:  1.0
Release:  1
Summary:  Sample RPM package
License:  BSD
URL:      https://github.com/rabbitmq/bintray-rs
Source0:  myapp_1.0.orig.tar.gz
BuildArch: noarch

%description
This sample package is used for the Bintray Rust crate testsuite.

%prep
%autosetup

%build

%install
mkdir -p %{buildroot}

%files
%doc README.md

%changelog
* Thu Jul 26 2018 RabbitMQ Team <info@rabbitmq.com> - 1.0-1
- Initial version of the package

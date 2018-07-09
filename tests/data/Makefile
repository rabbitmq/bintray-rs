.PHONY: all clean debian rpm maven

DEBIAN_DIR := $(CURDIR)/debian-package
RPM_DIR := $(CURDIR)/rpm-package
MAVEN_DIR := $(CURDIR)/maven-package

all: debian rpm maven
	@:

clean:
	rm -f \
		myapp_1.0-1_all.deb \
		myapp_1.0-1_amd64.buildinfo \
		myapp_1.0-1_amd64.changes \
		myapp_1.0-1.debian.tar.xz \
		myapp_1.0-1.dsc \
		myapp_1.0.orig.tar.gz
	rm -rf \
		myapp-1.0-1.noarch.rpm \
		$(RPM_DIR)/BUILD \
		$(RPM_DIR)/BUILDROOT \
		$(RPM_DIR)/RPMS \
		$(RPM_DIR)/SOURCES \
		$(RPM_DIR)/SPECS \
		$(RPM_DIR)/SRPMS \
		$(RPM_DIR)/tmp

# Debian package.

debian: myapp_1.0-1_all.deb
	@:

myapp_1.0.orig.tar.gz: README.md
	tar zcf "$@" --transform='s|^|myapp-1.0/|' $^

myapp_1.0-1_all.deb: myapp_1.0.orig.tar.gz
	cd $(DEBIAN_DIR) && \
	tar zxf ../$< --strip-components=1 && \
	dpkg-buildpackage -sa -us -uc && lintian
	rm -f \
		myapp_1.0-1_amd64.buildinfo \
		myapp_1.0-1_amd64.changes \
		myapp_1.0-1.debian.tar.xz \
		myapp_1.0-1.dsc

# RPM package.

rpm: myapp-1.0-1.noarch.rpm
	@:

myapp-1.0-1.noarch.rpm: myapp_1.0.orig.tar.gz
	cd $(RPM_DIR) && \
	mkdir -p BUILD RPMS SOURCES tmp && \
	cp -a ../$< SOURCES && \
	rpmbuild -bb \
		--define '_topdir $(RPM_DIR)' \
		--define '_tmppath $(RPM_DIR)/tmp' \
		myapp.spec
	mv $(RPM_DIR)/RPMS/noarch/$@ $@
	rm -rf \
		$(RPM_DIR)/BUILD \
		$(RPM_DIR)/BUILDROOT \
		$(RPM_DIR)/RPMS \
		$(RPM_DIR)/SOURCES \
		$(RPM_DIR)/SPECS \
		$(RPM_DIR)/SRPMS \
		$(RPM_DIR)/tmp

# Maven package.

maven: myapp-1.0.jar
	@:

myapp-1.0.jar:
	cd $(MAVEN_DIR) && \
	mvn package
	mv $(MAVEN_DIR)/target/$@ $@
	cd $(MAVEN_DIR) && \
	mvn clean
